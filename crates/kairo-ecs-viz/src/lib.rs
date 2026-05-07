#![forbid(unsafe_code)]

//! Optional visualization snapshot facade for KairoECS.
//!
//! This crate is intentionally headless-safe by default. The default build
//! contains the frame contract, deterministic summaries, and renderer
//! availability checks without linking GUI, windowing, WGPU, or Bevy
//! dependencies.

use std::error::Error;
use std::fmt::{Display, Formatter};

use kairo_ecs_state::WorldSnapshot;
use kairo_ecs_types::{DispatchedEvent, EntityId, EventKind, SimTime};

pub const WGPU_RENDERER_NOT_CONFIGURED: &str =
    "wgpu renderer feature is enabled, but native renderer dependencies are not configured";
pub const BEVY_RENDERER_NOT_CONFIGURED: &str =
    "bevy renderer feature is enabled, but Bevy app dependencies are not configured";
pub const VISUALIZATION_FIXTURE_SCHEMA: &str = "kairo_ecs.visualization.frame.v1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RendererStatus {
    HeadlessOnly,
    WgpuFeatureEnabledButNotConfigured,
    BevyFeatureEnabledButNotConfigured,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrameEntity {
    pub id: EntityId,
    pub label: String,
    pub x_milli: i64,
    pub y_milli: i64,
}

impl FrameEntity {
    pub fn new(id: EntityId, label: impl Into<String>, x_milli: i64, y_milli: i64) -> Self {
        Self {
            id,
            label: label.into(),
            x_milli,
            y_milli,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventMarker {
    pub at: SimTime,
    pub sequence: u64,
    pub kind: EventKind,
    pub entity: Option<EntityId>,
}

impl From<DispatchedEvent> for EventMarker {
    fn from(event: DispatchedEvent) -> Self {
        Self {
            at: event.at,
            sequence: event.sequence,
            kind: event.kind,
            entity: event.entity,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderFrame {
    pub at: SimTime,
    pub entities: Vec<FrameEntity>,
    pub events: Vec<EventMarker>,
}

impl RenderFrame {
    pub fn new(at: SimTime) -> Self {
        Self {
            at,
            entities: Vec::new(),
            events: Vec::new(),
        }
    }

    pub fn from_world_snapshot(at: SimTime, snapshot: &WorldSnapshot) -> Self {
        snapshot
            .entities()
            .iter()
            .enumerate()
            .fold(Self::new(at), |frame, (index, entity)| {
                frame.with_entity(FrameEntity::new(
                    entity.id,
                    format!("entity-{}", entity.id.index),
                    index as i64 * 1_000,
                    0,
                ))
            })
    }

    pub fn from_world_snapshot_and_events(
        at: SimTime,
        snapshot: &WorldSnapshot,
        events: impl IntoIterator<Item = DispatchedEvent>,
    ) -> Self {
        events
            .into_iter()
            .map(EventMarker::from)
            .fold(Self::from_world_snapshot(at, snapshot), |frame, event| {
                frame.with_event(event)
            })
    }

    pub fn with_entity(mut self, entity: FrameEntity) -> Self {
        self.entities.push(entity);
        self
    }

    pub fn with_event(mut self, event: EventMarker) -> Self {
        self.events.push(event);
        self
    }

    pub fn validate(&self) -> Result<(), VizError> {
        for entity in &self.entities {
            if entity.label.trim().is_empty() {
                return Err(VizError::new("frame entity label must not be empty"));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrameSummary {
    pub at_ticks: u128,
    pub entity_count: usize,
    pub event_count: usize,
    pub bounds: Option<FrameBounds>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VisualizationFixture {
    pub schema: &'static str,
    pub frame: RenderFrame,
    pub summary: FrameSummary,
}

impl VisualizationFixture {
    pub fn from_frame(frame: RenderFrame) -> Result<Self, VizError> {
        let summary = summarize_frame(&frame)?;
        Ok(Self {
            schema: VISUALIZATION_FIXTURE_SCHEMA,
            frame,
            summary,
        })
    }

    pub fn to_json(&self) -> String {
        let mut json = String::new();
        json.push_str("{\n");
        json.push_str(&format!("  \"schema\": \"{}\",\n", self.schema));
        json.push_str(&format!("  \"at_ticks\": \"{}\",\n", self.summary.at_ticks));
        json.push_str(&format!(
            "  \"entity_count\": {},\n",
            self.summary.entity_count
        ));
        json.push_str(&format!(
            "  \"event_count\": {},\n",
            self.summary.event_count
        ));
        json.push_str("  \"bounds\": ");
        match self.summary.bounds {
            Some(bounds) => json.push_str(&format!(
                "{{\"min_x_milli\": {}, \"min_y_milli\": {}, \"max_x_milli\": {}, \"max_y_milli\": {}}}",
                bounds.min_x_milli, bounds.min_y_milli, bounds.max_x_milli, bounds.max_y_milli
            )),
            None => json.push_str("null"),
        }
        json.push_str(",\n  \"entities\": [\n");
        for (index, entity) in self.frame.entities.iter().enumerate() {
            if index > 0 {
                json.push_str(",\n");
            }
            json.push_str(&format!(
                "    {{\"id\": \"{}:{}\", \"label\": \"{}\", \"x_milli\": {}, \"y_milli\": {}}}",
                entity.id.index,
                entity.id.generation,
                escape_json(&entity.label),
                entity.x_milli,
                entity.y_milli
            ));
        }
        json.push_str("\n  ],\n  \"events\": [\n");
        for (index, event) in self.frame.events.iter().enumerate() {
            if index > 0 {
                json.push_str(",\n");
            }
            json.push_str(&format!(
                "    {{\"at_ticks\": \"{}\", \"sequence\": {}, \"kind\": {}, \"entity\": {}}}",
                event.at.ticks(),
                event.sequence,
                event.kind.code(),
                event
                    .entity
                    .map(|entity| format!("\"{}:{}\"", entity.index, entity.generation))
                    .unwrap_or_else(|| "null".to_string())
            ));
        }
        json.push_str("\n  ]\n}");
        json
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameBounds {
    pub min_x_milli: i64,
    pub min_y_milli: i64,
    pub max_x_milli: i64,
    pub max_y_milli: i64,
}

pub fn summarize_frame(frame: &RenderFrame) -> Result<FrameSummary, VizError> {
    frame.validate()?;

    let bounds = frame
        .entities
        .iter()
        .fold(None::<FrameBounds>, |bounds, entity| {
            Some(match bounds {
                None => FrameBounds {
                    min_x_milli: entity.x_milli,
                    min_y_milli: entity.y_milli,
                    max_x_milli: entity.x_milli,
                    max_y_milli: entity.y_milli,
                },
                Some(bounds) => FrameBounds {
                    min_x_milli: bounds.min_x_milli.min(entity.x_milli),
                    min_y_milli: bounds.min_y_milli.min(entity.y_milli),
                    max_x_milli: bounds.max_x_milli.max(entity.x_milli),
                    max_y_milli: bounds.max_y_milli.max(entity.y_milli),
                },
            })
        });

    Ok(FrameSummary {
        at_ticks: frame.at.ticks(),
        entity_count: frame.entities.len(),
        event_count: frame.events.len(),
        bounds,
    })
}

pub fn renderer_statuses() -> Vec<RendererStatus> {
    let mut statuses = Vec::new();

    #[cfg(feature = "wgpu-renderer")]
    statuses.push(RendererStatus::WgpuFeatureEnabledButNotConfigured);

    #[cfg(feature = "bevy-renderer")]
    statuses.push(RendererStatus::BevyFeatureEnabledButNotConfigured);

    if statuses.is_empty() {
        statuses.push(RendererStatus::HeadlessOnly);
    }

    statuses
}

pub fn render_headless(frame: &RenderFrame) -> Result<FrameSummary, VizError> {
    summarize_frame(frame)
}

pub fn render_headless_text(frame: &RenderFrame) -> Result<String, VizError> {
    let summary = summarize_frame(frame)?;
    let mut lines = vec![format!(
        "frame ticks={} entities={} events={}",
        summary.at_ticks, summary.entity_count, summary.event_count
    )];

    for entity in &frame.entities {
        lines.push(format!(
            "entity id={}:{} label={} x_milli={} y_milli={}",
            entity.id.index, entity.id.generation, entity.label, entity.x_milli, entity.y_milli
        ));
    }

    for event in &frame.events {
        lines.push(format!(
            "event ticks={} sequence={} kind={} entity={}",
            event.at.ticks(),
            event.sequence,
            event.kind.code(),
            event
                .entity
                .map(|entity| format!("{}:{}", entity.index, entity.generation))
                .unwrap_or_else(|| "none".to_string())
        ));
    }

    Ok(lines.join("\n"))
}

pub fn render_fixture_json(frame: RenderFrame) -> Result<String, VizError> {
    VisualizationFixture::from_frame(frame).map(|fixture| fixture.to_json())
}

pub fn render_headless_svg(frame: &RenderFrame) -> Result<String, VizError> {
    let summary = summarize_frame(frame)?;
    let bounds = summary.bounds.unwrap_or(FrameBounds {
        min_x_milli: 0,
        min_y_milli: 0,
        max_x_milli: 1_000,
        max_y_milli: 1_000,
    });
    let width = (bounds.max_x_milli - bounds.min_x_milli).max(1_000) + 2_000;
    let height = (bounds.max_y_milli - bounds.min_y_milli).max(1_000) + 2_000;
    let offset_x = 1_000 - bounds.min_x_milli;
    let offset_y = 1_000 - bounds.min_y_milli;

    let mut svg = String::new();
    svg.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {} {}\" role=\"img\" aria-label=\"KairoECS frame at tick {}\">\n",
        width,
        height,
        frame.at.ticks()
    ));
    svg.push_str("  <rect width=\"100%\" height=\"100%\" fill=\"#f8fafc\"/>\n");
    svg.push_str("  <g fill=\"#0f766e\" stroke=\"#134e4a\" stroke-width=\"40\">\n");
    for entity in &frame.entities {
        svg.push_str(&format!(
            "    <circle cx=\"{}\" cy=\"{}\" r=\"180\"><title>{}</title></circle>\n",
            entity.x_milli + offset_x,
            entity.y_milli + offset_y,
            escape_xml(&entity.label)
        ));
    }
    svg.push_str("  </g>\n  <g font-family=\"monospace\" font-size=\"220\" fill=\"#111827\">\n");
    for entity in &frame.entities {
        svg.push_str(&format!(
            "    <text x=\"{}\" y=\"{}\">{}</text>\n",
            entity.x_milli + offset_x + 260,
            entity.y_milli + offset_y + 80,
            escape_xml(&entity.label)
        ));
    }
    svg.push_str("  </g>\n</svg>\n");
    Ok(svg)
}

fn escape_json(value: &str) -> String {
    value
        .chars()
        .flat_map(|ch| match ch {
            '"' => "\\\"".chars().collect::<Vec<_>>(),
            '\\' => "\\\\".chars().collect::<Vec<_>>(),
            '\n' => "\\n".chars().collect::<Vec<_>>(),
            '\r' => "\\r".chars().collect::<Vec<_>>(),
            '\t' => "\\t".chars().collect::<Vec<_>>(),
            _ => vec![ch],
        })
        .collect()
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(feature = "wgpu-renderer")]
pub mod wgpu_renderer {
    use crate::{RenderFrame, VizError, WGPU_RENDERER_NOT_CONFIGURED};

    pub fn render_frame(_frame: &RenderFrame) -> Result<(), VizError> {
        Err(VizError::new(WGPU_RENDERER_NOT_CONFIGURED))
    }
}

#[cfg(feature = "bevy-renderer")]
pub mod bevy_renderer {
    use crate::{RenderFrame, VizError, BEVY_RENDERER_NOT_CONFIGURED};

    pub fn render_frame(_frame: &RenderFrame) -> Result<(), VizError> {
        Err(VizError::new(BEVY_RENDERER_NOT_CONFIGURED))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VizError {
    message: String,
}

impl VizError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Display for VizError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for VizError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn entity(index: u64) -> EntityId {
        EntityId {
            index,
            generation: 0,
        }
    }

    #[test]
    fn default_renderer_status_is_headless_only() {
        #[cfg(not(any(feature = "wgpu-renderer", feature = "bevy-renderer")))]
        assert_eq!(renderer_statuses(), vec![RendererStatus::HeadlessOnly]);
    }

    #[test]
    fn summarizes_frame_without_gui_dependencies() {
        let frame = RenderFrame::new(SimTime::from_ticks(42))
            .with_entity(FrameEntity::new(entity(1), "queue", -100, 250))
            .with_entity(FrameEntity::new(entity(2), "server", 900, -50))
            .with_event(EventMarker {
                at: SimTime::from_ticks(42),
                sequence: 7,
                kind: EventKind::Custom(3),
                entity: Some(entity(1)),
            });

        let summary = render_headless(&frame).expect("headless summary");

        assert_eq!(summary.at_ticks, 42);
        assert_eq!(summary.entity_count, 2);
        assert_eq!(summary.event_count, 1);
        assert_eq!(
            summary.bounds,
            Some(FrameBounds {
                min_x_milli: -100,
                min_y_milli: -50,
                max_x_milli: 900,
                max_y_milli: 250,
            })
        );
    }

    #[test]
    fn converts_world_snapshot_to_deterministic_headless_frame() {
        let mut world = kairo_ecs_state::World::new();
        world.spawn();
        world.spawn();

        let frame = RenderFrame::from_world_snapshot(SimTime::from_ticks(12), &world.snapshot());
        let summary = render_headless(&frame).expect("headless summary");

        assert_eq!(summary.at_ticks, 12);
        assert_eq!(summary.entity_count, 2);
        assert_eq!(summary.event_count, 0);
        assert_eq!(
            summary.bounds,
            Some(FrameBounds {
                min_x_milli: 0,
                min_y_milli: 0,
                max_x_milli: 1_000,
                max_y_milli: 0,
            })
        );
    }

    #[test]
    fn converts_dispatched_events_to_headless_markers() {
        let mut world = kairo_ecs_state::World::new();
        let entity = world.spawn();
        let event = DispatchedEvent::new(
            kairo_ecs_types::EventId {
                index: 9,
                generation: 0,
            },
            SimTime::from_ticks(32),
            0,
            4,
            Some(entity),
            EventKind::Custom(77),
        );

        let frame = RenderFrame::from_world_snapshot_and_events(
            SimTime::from_ticks(32),
            &world.snapshot(),
            [event],
        );

        let summary = render_headless(&frame).expect("headless summary");
        assert_eq!(summary.entity_count, 1);
        assert_eq!(summary.event_count, 1);
        assert_eq!(
            frame.events,
            vec![EventMarker {
                at: SimTime::from_ticks(32),
                sequence: 4,
                kind: EventKind::Custom(77),
                entity: Some(entity),
            }]
        );
    }

    #[test]
    fn renders_deterministic_headless_text() {
        let frame = RenderFrame::new(SimTime::from_ticks(3))
            .with_entity(FrameEntity::new(entity(2), "server", 100, 200))
            .with_event(EventMarker {
                at: SimTime::from_ticks(3),
                sequence: 1,
                kind: EventKind::Custom(5),
                entity: Some(entity(2)),
            });

        let text = render_headless_text(&frame).expect("headless text");

        assert_eq!(
            text,
            "frame ticks=3 entities=1 events=1\nentity id=2:0 label=server x_milli=100 y_milli=200\nevent ticks=3 sequence=1 kind=5 entity=2:0"
        );
    }

    #[test]
    fn renders_formal_visualization_fixture_json() {
        let frame = RenderFrame::new(SimTime::from_ticks(5))
            .with_entity(FrameEntity::new(entity(1), "queue \"a\"", 0, 0))
            .with_event(EventMarker {
                at: SimTime::from_ticks(5),
                sequence: 2,
                kind: EventKind::Custom(8),
                entity: None,
            });

        let json = render_fixture_json(frame).expect("fixture json");

        assert_eq!(
            json,
            "{\n  \"schema\": \"kairo_ecs.visualization.frame.v1\",\n  \"at_ticks\": \"5\",\n  \"entity_count\": 1,\n  \"event_count\": 1,\n  \"bounds\": {\"min_x_milli\": 0, \"min_y_milli\": 0, \"max_x_milli\": 0, \"max_y_milli\": 0},\n  \"entities\": [\n    {\"id\": \"1:0\", \"label\": \"queue \\\"a\\\"\", \"x_milli\": 0, \"y_milli\": 0}\n  ],\n  \"events\": [\n    {\"at_ticks\": \"5\", \"sequence\": 2, \"kind\": 8, \"entity\": null}\n  ]\n}"
        );
    }

    #[test]
    fn renders_deterministic_headless_svg() {
        let frame = RenderFrame::new(SimTime::from_ticks(9)).with_entity(FrameEntity::new(
            entity(3),
            "server <one>",
            500,
            750,
        ));

        let svg = render_headless_svg(&frame).expect("svg");

        assert!(svg.contains("KairoECS frame at tick 9"));
        assert!(svg.contains("<circle cx=\"1000\" cy=\"1000\" r=\"180\">"));
        assert!(svg.contains("server &lt;one&gt;"));
    }

    #[test]
    fn rejects_blank_entity_labels() {
        let frame =
            RenderFrame::new(SimTime::ZERO).with_entity(FrameEntity::new(entity(1), " ", 0, 0));

        assert_eq!(
            render_headless(&frame)
                .expect_err("blank label should fail")
                .to_string(),
            "frame entity label must not be empty"
        );
    }
}
