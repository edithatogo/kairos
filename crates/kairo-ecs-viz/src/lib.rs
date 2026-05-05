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
use kairo_ecs_types::{EntityId, EventKind, SimTime};

pub const WGPU_RENDERER_NOT_CONFIGURED: &str =
    "wgpu renderer feature is enabled, but native renderer dependencies are not configured";
pub const BEVY_RENDERER_NOT_CONFIGURED: &str =
    "bevy renderer feature is enabled, but Bevy app dependencies are not configured";

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
