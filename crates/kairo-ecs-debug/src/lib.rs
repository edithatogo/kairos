#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};

use kairo_ecs_core::{RecordedEvent, RecordingScheduler, Scheduler};
use kairo_ecs_types::{
    DispatchedEvent, EntityId, EventId, EventKind, ScheduleRequest, SimTime, StepOutcome,
};

pub const TRACE_SCHEMA: &str = "kairo.ecs.trace.v1";

pub const SNAPSHOT_FIELD_COUNT: usize = 3;
pub const DELTA_FIELD_COUNT: usize = 7;

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
            .map(|snapshot| {
                snapshot
                    .state
                    .iter()
                    .collect::<BTreeMap<&String, &String>>()
            })
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
                state.insert(key, value);
            }
        }
        state
            .into_iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
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
        let parts = line.split('\t').collect::<Vec<_>>();
        let kind = parts
            .first()
            .ok_or_else(|| TraceValidationError::MalformedLine(line.to_string()))?;
        let tick = parts
            .get(1)
            .ok_or_else(|| TraceValidationError::MalformedLine(line.to_string()))?
            .parse::<u128>()
            .map_err(|_| TraceValidationError::MalformedLine(line.to_string()))?;
        match *kind {
            "snapshot" if parts.len() == 3 => {}
            "delta" if parts.len() == 7 => {
                parts[2]
                    .parse::<u64>()
                    .map_err(|_| TraceValidationError::MalformedLine(line.to_string()))?;
                parts[3]
                    .parse::<u32>()
                    .map_err(|_| TraceValidationError::MalformedLine(line.to_string()))?;
                parts[4]
                    .parse::<u64>()
                    .map_err(|_| TraceValidationError::MalformedLine(line.to_string()))?;
                if parse_encoded_event_kind(parts[5]).is_none() {
                    return Err(TraceValidationError::MalformedLine(line.to_string()));
                }
            }
            _ => return Err(TraceValidationError::MalformedLine(line.to_string())),
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
    cursor: Option<usize>,
    breakpoints: Vec<Breakpoint>,
}

impl Debugger {
    pub fn new(trace: EventTrace) -> Self {
        Self {
            trace,
            cursor: None,
            breakpoints: Vec::new(),
        }
    }

    pub fn step(&mut self) -> Result<&TraceDelta, DebuggerError> {
        if self.trace.deltas.is_empty() {
            return Err(DebuggerError::EmptyTrace);
        }
        let next = self
            .cursor
            .map(|cursor| (cursor + 1).min(self.trace.deltas.len() - 1))
            .unwrap_or(0);
        self.cursor = Some(next);
        Ok(&self.trace.deltas[next])
    }

    pub fn back(&mut self) -> Result<&TraceDelta, DebuggerError> {
        if self.trace.deltas.is_empty() {
            return Err(DebuggerError::EmptyTrace);
        }
        let previous = self.cursor.unwrap_or(0).saturating_sub(1);
        self.cursor = Some(previous);
        Ok(&self.trace.deltas[previous])
    }

    pub fn goto_tick(&mut self, tick: u128) -> Result<&TraceDelta, DebuggerError> {
        let index = self
            .trace
            .deltas
            .iter()
            .position(|delta| delta.tick >= tick)
            .ok_or(DebuggerError::TickNotFound(tick))?;
        self.cursor = Some(index);
        Ok(&self.trace.deltas[index])
    }

    pub fn inspect(&self, key: &str) -> Option<String> {
        let tick = self
            .cursor
            .and_then(|cursor| self.trace.deltas.get(cursor))
            .map(|delta| delta.tick)
            .unwrap_or(0);
        self.trace.reconstruct_at(tick).get(key).cloned()
    }

    pub fn cursor_tick(&self) -> u128 {
        self.cursor
            .and_then(|cursor| self.trace.deltas.get(cursor))
            .map(|delta| delta.tick)
            .unwrap_or(0)
    }

    pub fn current_state(&self) -> BTreeMap<String, String> {
        self.trace.reconstruct_at(self.cursor_tick())
    }

    pub fn add_breakpoint(&mut self, breakpoint: Breakpoint) {
        self.breakpoints.push(breakpoint);
    }

    pub fn list_breakpoints(&self) -> &[Breakpoint] {
        &self.breakpoints
    }

    pub fn next_breakpoint(&self) -> Option<&TraceDelta> {
        let start = self.cursor.map(|cursor| cursor + 1).unwrap_or(0);
        self.trace.deltas.iter().skip(start).find(|delta| {
            self.breakpoints.iter().any(|breakpoint| match breakpoint {
                Breakpoint::EventKind(kind) => &delta.kind == kind,
                Breakpoint::Entity(entity) => delta.entity.as_ref() == Some(entity),
            })
        })
    }

    pub fn run_until_breakpoint(&mut self) -> Option<&TraceDelta> {
        let start = self.cursor.map(|cursor| cursor + 1).unwrap_or(0);
        let index = self
            .trace
            .deltas
            .iter()
            .enumerate()
            .skip(start)
            .find(|(_, delta)| {
                self.breakpoints.iter().any(|breakpoint| match breakpoint {
                    Breakpoint::EventKind(kind) => &delta.kind == kind,
                    Breakpoint::Entity(entity) => delta.entity.as_ref() == Some(entity),
                })
            })
            .map(|(index, _)| index)?;
        self.cursor = Some(index);
        self.trace.deltas.get(index)
    }
}

/// Standalone recorder that wraps `&mut Scheduler` and records events identically
/// to `RecordingScheduler` for use cases where ownership of the scheduler must remain elsewhere.
#[derive(Debug)]
pub struct TraceRecorder<'a> {
    pub scheduler: &'a mut Scheduler,
    pub recorded: Vec<RecordedEvent>,
}

impl<'a> TraceRecorder<'a> {
    pub fn new(scheduler: &'a mut Scheduler) -> Self {
        Self {
            scheduler,
            recorded: Vec::new(),
        }
    }

    pub fn schedule(&mut self, req: ScheduleRequest) -> EventId {
        self.scheduler.schedule(req)
    }

    pub fn cancel(&mut self, id: EventId) -> bool {
        self.scheduler.cancel(id)
    }

    pub fn step(&mut self) -> StepOutcome {
        let outcome = self.scheduler.step();
        if let StepOutcome::Dispatched(ref ev) = outcome {
            self.recorded.push(RecordedEvent {
                tick: ev.at.ticks() as u64,
                event_id: ev.id.index,
                entity_id: ev.entity.map(|e| e.index),
                priority: ev.priority,
                sequence: ev.sequence,
                kind: match ev.kind {
                    EventKind::Custom(v) => v,
                },
            });
        }
        outcome
    }

    pub fn run_for(&mut self, max_events: u64) -> u64 {
        let mut count = 0;
        while count < max_events {
            match self.step() {
                StepOutcome::Dispatched(_) => count += 1,
                _ => break,
            }
        }
        count
    }

    pub fn run_until(&mut self, time_limit: SimTime) -> u64 {
        let mut count = 0;
        while let StepOutcome::Dispatched(ref ev) = self.step() {
            if ev.at >= time_limit {
                break;
            }
            count += 1;
        }
        count
    }

    pub fn pending_events(&self) -> usize {
        self.scheduler.pending_events()
    }

    pub fn now(&self) -> SimTime {
        self.scheduler.now()
    }
}

/// Replays recorded events with step-by-step assertions for deterministic testing.
#[derive(Clone, Debug)]
pub struct TraceReplay {
    events: Vec<RecordedEvent>,
    cursor: usize,
}

impl TraceReplay {
    pub fn new(events: Vec<RecordedEvent>) -> Self {
        Self { events, cursor: 0 }
    }

    pub fn from_recorder(recorder: &TraceRecorder) -> Self {
        Self::new(recorder.recorded.clone())
    }

    pub fn from_recording(recording: &RecordingScheduler) -> Self {
        Self::new(recording.recorded.clone())
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn remaining(&self) -> usize {
        self.events.len().saturating_sub(self.cursor)
    }

    pub fn reset(&mut self) {
        self.cursor = 0;
    }

    pub fn step(&mut self) -> Option<&RecordedEvent> {
        let event = self.events.get(self.cursor)?;
        self.cursor += 1;
        Some(event)
    }

    pub fn peek(&self) -> Option<&RecordedEvent> {
        self.events.get(self.cursor)
    }

    pub fn peek_at(&self, index: usize) -> Option<&RecordedEvent> {
        self.events.get(index)
    }

    pub fn assert_count(&self, expected: usize) -> &Self {
        assert_eq!(
            self.events.len(),
            expected,
            "expected {expected} recorded events, got {}",
            self.events.len()
        );
        self
    }

    pub fn assert_tick(&self, index: usize, expected: u64) -> &Self {
        let event = self
            .events
            .get(index)
            .unwrap_or_else(|| panic!("no event at index {index}"));
        assert_eq!(
            event.tick, expected,
            "event[{index}] tick mismatch: expected {expected}, got {}",
            event.tick
        );
        self
    }

    pub fn assert_kind(&self, index: usize, expected: u32) -> &Self {
        let event = self
            .events
            .get(index)
            .unwrap_or_else(|| panic!("no event at index {index}"));
        assert_eq!(
            event.kind, expected,
            "event[{index}] kind mismatch: expected {expected}, got {}",
            event.kind
        );
        self
    }

    pub fn assert_entity(&self, index: usize, expected: Option<u64>) -> &Self {
        let event = self
            .events
            .get(index)
            .unwrap_or_else(|| panic!("no event at index {index}"));
        assert_eq!(
            event.entity_id, expected,
            "event[{index}] entity mismatch: expected {:?}, got {:?}",
            expected, event.entity_id
        );
        self
    }

    pub fn assert_sequence(&self, index: usize, expected: u64) -> &Self {
        let event = self
            .events
            .get(index)
            .unwrap_or_else(|| panic!("no event at index {index}"));
        assert_eq!(
            event.sequence, expected,
            "event[{index}] sequence mismatch: expected {expected}, got {}",
            event.sequence
        );
        self
    }

    pub fn assert_all_dispatched(&mut self) -> &Self {
        while self.step().is_some() {}
        assert!(
            self.cursor == self.events.len(),
            "did not step through all events"
        );
        self
    }
}

impl IntoIterator for TraceReplay {
    type Item = RecordedEvent;
    type IntoIter = std::vec::IntoIter<RecordedEvent>;

    fn into_iter(self) -> Self::IntoIter {
        self.events.into_iter()
    }
}

fn encode_kind(kind: &EventKind) -> String {
    match kind {
        EventKind::Custom(value) => format!("custom:{value}"),
    }
}

fn parse_encoded_event_kind(value: &str) -> Option<EventKind> {
    let raw = value.strip_prefix("custom:")?;
    raw.parse::<u32>().ok().map(EventKind::Custom)
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

    fn machine_trace() -> EventTrace {
        let mut trace = EventTrace::default();
        let mut initial = BTreeMap::new();
        initial.insert("machine.status".to_string(), "idle".to_string());
        trace.snapshot(SimTime::from_ticks(0), initial);

        let mut queued = BTreeMap::new();
        queued.insert("machine.status".to_string(), "queued".to_string());
        trace.record_event(event(2, 1, 0), queued);

        let mut busy = BTreeMap::new();
        busy.insert("machine.status".to_string(), "busy".to_string());
        trace.record_event(event(4, 9, 1), busy);

        trace
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
        let mut debugger = Debugger::new(machine_trace());

        assert_eq!(debugger.cursor_tick(), 0);
        assert_eq!(debugger.inspect("machine.status").as_deref(), Some("idle"));
        assert_eq!(debugger.step().unwrap().tick, 2);
        assert_eq!(
            debugger.current_state()["machine.status"],
            "queued",
            "first step should move from the initial snapshot to the first delta"
        );
        assert_eq!(debugger.step().unwrap().tick, 4);
        assert_eq!(debugger.back().unwrap().tick, 2);
        assert_eq!(debugger.goto_tick(3).unwrap().tick, 4);
        assert_eq!(debugger.inspect("machine.status").as_deref(), Some("busy"));
    }

    #[test]
    fn breakpoint_matches_event_kind() {
        let mut debugger = Debugger::new(machine_trace());
        debugger.add_breakpoint(Breakpoint::EventKind(EventKind::Custom(9)));

        assert_eq!(debugger.next_breakpoint().unwrap().tick, 4);
        assert_eq!(debugger.run_until_breakpoint().unwrap().tick, 4);
        assert_eq!(debugger.inspect("machine.status").as_deref(), Some("busy"));
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

    #[test]
    fn trace_line_validation_rejects_missing_or_unsupported_schema() {
        assert_eq!(
            validate_trace_lines(""),
            Err(TraceValidationError::MissingSchema)
        );
        assert_eq!(
            validate_trace_lines("schema\tkairo.ecs.trace.v0\n"),
            Err(TraceValidationError::UnsupportedSchema(
                "kairo.ecs.trace.v0".to_string()
            ))
        );
    }

    #[test]
    fn trace_line_validation_rejects_malformed_delta_fields() {
        let malformed_event_id = concat!(
            "schema\tkairo.ecs.trace.v1\n",
            "snapshot\t0\t\n",
            "delta\t2\tnot-an-id\t0\t0\tcustom:1\t\n",
        );
        let missing_delta_map = concat!(
            "schema\tkairo.ecs.trace.v1\n",
            "snapshot\t0\t\n",
            "delta\t2\t0\t0\t0\tcustom:1\n",
        );

        assert!(matches!(
            validate_trace_lines(malformed_event_id),
            Err(TraceValidationError::MalformedLine(_))
        ));
        assert!(matches!(
            validate_trace_lines(missing_delta_map),
            Err(TraceValidationError::MalformedLine(_))
        ));
    }

    #[test]
    fn trace_line_validation_rejects_malformed_custom_event_kind() {
        let malformed_kind = concat!(
            "schema\tkairo.ecs.trace.v1\n",
            "snapshot\t0\t\n",
            "delta\t2\t0\t0\t0\tcustom:not-a-u32\t\n",
        );
        let unsupported_kind = concat!(
            "schema\tkairo.ecs.trace.v1\n",
            "snapshot\t0\t\n",
            "delta\t2\t0\t0\t0\tdomain:arrival\t\n",
        );

        assert!(matches!(
            validate_trace_lines(malformed_kind),
            Err(TraceValidationError::MalformedLine(_))
        ));
        assert!(matches!(
            validate_trace_lines(unsupported_kind),
            Err(TraceValidationError::MalformedLine(_))
        ));
    }
}
