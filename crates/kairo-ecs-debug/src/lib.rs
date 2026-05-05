#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};

use kairo_ecs_types::{DispatchedEvent, EntityId, EventId, EventKind, SimTime};

pub const TRACE_SCHEMA: &str = "kairo.ecs.trace.v1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceSnapshot {
    pub tick: u128,
    pub state: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceDelta {
    pub tick: u128,
    pub event_id: EventId,
    pub priority: i32,
    pub sequence: u64,
    pub entity: Option<EntityId>,
    pub kind: EventKind,
    pub changes: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventTrace {
    pub schema: &'static str,
    pub snapshots: Vec<TraceSnapshot>,
    pub deltas: Vec<TraceDelta>,
}

impl Default for EventTrace {
    fn default() -> Self {
        Self {
            schema: TRACE_SCHEMA,
            snapshots: vec![TraceSnapshot {
                tick: 0,
                state: BTreeMap::new(),
            }],
            deltas: Vec::new(),
        }
    }
}

impl EventTrace {
    pub fn record_event(&mut self, event: DispatchedEvent, changes: BTreeMap<String, String>) {
        self.deltas.push(TraceDelta {
            tick: event.at.ticks(),
            event_id: event.id,
            priority: event.priority,
            sequence: event.sequence,
            entity: event.entity,
            kind: event.kind,
            changes,
        });
        self.deltas
            .sort_by_key(|delta| (delta.tick, delta.priority, delta.sequence));
    }

    pub fn snapshot(&mut self, tick: SimTime, state: BTreeMap<String, String>) {
        self.snapshots.push(TraceSnapshot {
            tick: tick.ticks(),
            state,
        });
        self.snapshots.sort_by_key(|snapshot| snapshot.tick);
    }

    pub fn reconstruct_at(&self, tick: u128) -> BTreeMap<String, String> {
        let mut state = self
            .snapshots
            .iter()
            .rev()
            .find(|snapshot| snapshot.tick <= tick)
            .map(|snapshot| snapshot.state.clone())
            .unwrap_or_default();

        let snapshot_tick = self
            .snapshots
            .iter()
            .rev()
            .find(|snapshot| snapshot.tick <= tick)
            .map(|snapshot| snapshot.tick)
            .unwrap_or(0);

        for delta in self
            .deltas
            .iter()
            .filter(|delta| delta.tick > snapshot_tick && delta.tick <= tick)
        {
            for (key, value) in &delta.changes {
                state.insert(key.clone(), value.clone());
            }
        }
        state
    }

    pub fn encode_lines(&self) -> String {
        let mut lines = vec![format!("schema\t{}", self.schema)];
        for snapshot in &self.snapshots {
            lines.push(format!(
                "snapshot\t{}\t{}",
                snapshot.tick,
                encode_map(&snapshot.state)
            ));
        }
        for delta in &self.deltas {
            lines.push(format!(
                "delta\t{}\t{}\t{}\t{}\t{}\t{}",
                delta.tick,
                delta.event_id.index,
                delta.event_id.generation,
                delta.sequence,
                encode_kind(&delta.kind),
                encode_map(&delta.changes)
            ));
        }
        lines.join("\n") + "\n"
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Breakpoint {
    EventKind(EventKind),
    Entity(EntityId),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DebuggerError {
    EmptyTrace,
    TickNotFound(u128),
}

impl Display for DebuggerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyTrace => write!(f, "trace contains no deltas"),
            Self::TickNotFound(tick) => write!(f, "tick not found: {tick}"),
        }
    }
}

impl std::error::Error for DebuggerError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TraceValidationError {
    MissingSchema,
    UnsupportedSchema(String),
    TickOutOfOrder { previous: u128, current: u128 },
    MalformedLine(String),
}

impl Display for TraceValidationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingSchema => write!(f, "trace is missing schema header"),
            Self::UnsupportedSchema(schema) => write!(f, "unsupported trace schema: {schema}"),
            Self::TickOutOfOrder { previous, current } => {
                write!(f, "trace ticks out of order: {current} after {previous}")
            }
            Self::MalformedLine(line) => write!(f, "malformed trace line: {line}"),
        }
    }
}

impl std::error::Error for TraceValidationError {}

pub fn validate_trace_lines(input: &str) -> Result<(), TraceValidationError> {
    let mut lines = input.lines();
    let schema = lines.next().ok_or(TraceValidationError::MissingSchema)?;
    if schema != format!("schema\t{TRACE_SCHEMA}") {
        let actual = schema
            .strip_prefix("schema\t")
            .unwrap_or(schema)
            .to_string();
        return Err(TraceValidationError::UnsupportedSchema(actual));
    }

    let mut previous_tick = 0;
    for line in lines {
        let mut parts = line.split('\t');
        let kind = parts
            .next()
            .ok_or_else(|| TraceValidationError::MalformedLine(line.to_string()))?;
        let tick = parts
            .next()
            .ok_or_else(|| TraceValidationError::MalformedLine(line.to_string()))?
            .parse::<u128>()
            .map_err(|_| TraceValidationError::MalformedLine(line.to_string()))?;
        if kind != "snapshot" && kind != "delta" {
            return Err(TraceValidationError::MalformedLine(line.to_string()));
        }
        if tick < previous_tick {
            return Err(TraceValidationError::TickOutOfOrder {
                previous: previous_tick,
                current: tick,
            });
        }
        previous_tick = tick;
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub struct Debugger {
    trace: EventTrace,
    cursor: usize,
    breakpoints: Vec<Breakpoint>,
}

impl Debugger {
    pub fn new(trace: EventTrace) -> Self {
        Self {
            trace,
            cursor: 0,
            breakpoints: Vec::new(),
        }
    }

    pub fn step(&mut self) -> Result<&TraceDelta, DebuggerError> {
        if self.trace.deltas.is_empty() {
            return Err(DebuggerError::EmptyTrace);
        }
        if self.cursor + 1 < self.trace.deltas.len() {
            self.cursor += 1;
        }
        Ok(&self.trace.deltas[self.cursor])
    }

    pub fn back(&mut self) -> Result<&TraceDelta, DebuggerError> {
        if self.trace.deltas.is_empty() {
            return Err(DebuggerError::EmptyTrace);
        }
        self.cursor = self.cursor.saturating_sub(1);
        Ok(&self.trace.deltas[self.cursor])
    }

    pub fn goto_tick(&mut self, tick: u128) -> Result<&TraceDelta, DebuggerError> {
        let index = self
            .trace
            .deltas
            .iter()
            .position(|delta| delta.tick >= tick)
            .ok_or(DebuggerError::TickNotFound(tick))?;
        self.cursor = index;
        Ok(&self.trace.deltas[self.cursor])
    }

    pub fn inspect(&self, key: &str) -> Option<String> {
        let tick = self
            .trace
            .deltas
            .get(self.cursor)
            .map(|delta| delta.tick)
            .unwrap_or(0);
        self.trace.reconstruct_at(tick).get(key).cloned()
    }

    pub fn add_breakpoint(&mut self, breakpoint: Breakpoint) {
        self.breakpoints.push(breakpoint);
    }

    pub fn list_breakpoints(&self) -> &[Breakpoint] {
        &self.breakpoints
    }

    pub fn next_breakpoint(&self) -> Option<&TraceDelta> {
        self.trace.deltas.iter().skip(self.cursor).find(|delta| {
            self.breakpoints.iter().any(|breakpoint| match breakpoint {
                Breakpoint::EventKind(kind) => &delta.kind == kind,
                Breakpoint::Entity(entity) => delta.entity.as_ref() == Some(entity),
            })
        })
    }
}

fn encode_kind(kind: &EventKind) -> String {
    match kind {
        EventKind::Custom(value) => format!("custom:{value}"),
    }
}

fn encode_map(map: &BTreeMap<String, String>) -> String {
    map.iter()
        .map(|(key, value)| format!("{}={}", escape(key), escape(value)))
        .collect::<Vec<_>>()
        .join(";")
}

fn escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace(';', "\\;")
        .replace('=', "\\=")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(tick: u128, kind: u32, sequence: u64) -> DispatchedEvent {
        DispatchedEvent {
            id: EventId {
                index: sequence,
                generation: 0,
            },
            at: SimTime::from_ticks(tick),
            priority: 0,
            sequence,
            entity: Some(EntityId {
                index: 7,
                generation: 1,
            }),
            kind: EventKind::Custom(kind),
        }
    }

    #[test]
    fn replay_reconstructs_state_from_snapshot_and_deltas() {
        let mut trace = EventTrace::default();
        let mut initial = BTreeMap::new();
        initial.insert("machine.status".to_string(), "idle".to_string());
        trace.snapshot(SimTime::from_ticks(0), initial);

        let mut change = BTreeMap::new();
        change.insert("machine.status".to_string(), "busy".to_string());
        trace.record_event(event(5, 1, 0), change);

        assert_eq!(trace.reconstruct_at(0)["machine.status"], "idle");
        assert_eq!(trace.reconstruct_at(5)["machine.status"], "busy");
    }

    #[test]
    fn debugger_steps_back_and_goto_tick() {
        let mut trace = EventTrace::default();
        trace.record_event(event(2, 1, 0), BTreeMap::new());
        trace.record_event(event(4, 2, 1), BTreeMap::new());

        let mut debugger = Debugger::new(trace);
        assert_eq!(debugger.step().unwrap().tick, 4);
        assert_eq!(debugger.back().unwrap().tick, 2);
        assert_eq!(debugger.goto_tick(3).unwrap().tick, 4);
    }

    #[test]
    fn breakpoint_matches_event_kind() {
        let mut trace = EventTrace::default();
        trace.record_event(event(2, 1, 0), BTreeMap::new());
        trace.record_event(event(4, 9, 1), BTreeMap::new());
        let mut debugger = Debugger::new(trace);
        debugger.add_breakpoint(Breakpoint::EventKind(EventKind::Custom(9)));

        assert_eq!(debugger.next_breakpoint().unwrap().tick, 4);
        assert_eq!(debugger.list_breakpoints().len(), 1);
    }

    #[test]
    fn line_encoding_declares_schema() {
        let trace = EventTrace::default();
        assert!(trace
            .encode_lines()
            .starts_with("schema\tkairo.ecs.trace.v1\n"));
    }

    #[test]
    fn trace_line_validation_accepts_encoded_trace() {
        let mut trace = EventTrace::default();
        trace.record_event(event(2, 1, 0), BTreeMap::new());
        trace.record_event(event(4, 2, 1), BTreeMap::new());

        assert_eq!(validate_trace_lines(&trace.encode_lines()), Ok(()));
    }

    #[test]
    fn trace_line_validation_rejects_out_of_order_ticks() {
        let input = concat!(
            "schema\tkairo.ecs.trace.v1\n",
            "delta\t4\t0\t0\t0\tcustom:1\t\n",
            "delta\t2\t1\t0\t1\tcustom:2\t\n",
        );

        assert!(matches!(
            validate_trace_lines(input),
            Err(TraceValidationError::TickOutOfOrder {
                previous: 4,
                current: 2
            })
        ));
    }
}
