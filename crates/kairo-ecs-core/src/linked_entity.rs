#![forbid(unsafe_code)]

use kairo_ecs_types::{EntityId, SimTime};
use std::fmt;

pub const RELATIONSHIP_SCHEMA_VERSION: u16 = 1;
pub const RELATIONSHIP_STREAM_NAME: &str = "kairo.relationship.telemetry.v1";

/// Stable external model identity distinct from recycled generational `EntityId`.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ModelEntityId(pub u64);

impl ModelEntityId {
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

impl fmt::Display for ModelEntityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ModelId({})", self.0)
    }
}

/// Typed relationship kinds distinguishing semantic connection types.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RelationshipKind {
    ParentChild,
    SpawnedFrom,
    SubtaskOf,
    CargoOf,
    Custom(u32),
}

impl RelationshipKind {
    pub const fn code(self) -> u32 {
        match self {
            Self::ParentChild => 1,
            Self::SpawnedFrom => 2,
            Self::SubtaskOf => 3,
            Self::CargoOf => 4,
            Self::Custom(code) => code,
        }
    }

    pub const fn from_code(code: u32) -> Self {
        match code {
            1 => Self::ParentChild,
            2 => Self::SpawnedFrom,
            3 => Self::SubtaskOf,
            4 => Self::CargoOf,
            other => Self::Custom(other),
        }
    }
}

/// Explicit lifecycle policy for linked child entities upon parent termination or transition.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum LifecyclePolicy {
    /// Independent child survives parent termination (detaches to root).
    Survive,
    /// Child is cascaded and deleted when the parent is despawned.
    CascadeDelete,
    /// Explicit detachment when parent or child transitions.
    Detach,
}

impl LifecyclePolicy {
    pub const fn code(self) -> u8 {
        match self {
            Self::Survive => 0,
            Self::CascadeDelete => 1,
            Self::Detach => 2,
        }
    }

    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Survive),
            1 => Some(Self::CascadeDelete),
            2 => Some(Self::Detach),
            _ => None,
        }
    }
}

/// Typed parent-child binding specification.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ParentChildBinding {
    pub parent: EntityId,
    pub child: EntityId,
    pub kind: RelationshipKind,
    pub policy: LifecyclePolicy,
}

impl ParentChildBinding {
    pub const fn new(
        parent: EntityId,
        child: EntityId,
        kind: RelationshipKind,
        policy: LifecyclePolicy,
    ) -> Self {
        Self {
            parent,
            child,
            kind,
            policy,
        }
    }
}

/// Deterministic spawn relationship record linking parent and child.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SpawnRelationship {
    pub parent: EntityId,
    pub child: EntityId,
    pub parent_model_id: Option<ModelEntityId>,
    pub child_model_id: Option<ModelEntityId>,
    pub kind: RelationshipKind,
    pub policy: LifecyclePolicy,
    pub spawned_at: SimTime,
}

/// Relationship lifecycle events for telemetry and audit trails.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RelationshipLifecycleEvent {
    Attached {
        parent: EntityId,
        child: EntityId,
        kind: RelationshipKind,
        policy: LifecyclePolicy,
        at: SimTime,
    },
    Detached {
        parent: EntityId,
        child: EntityId,
        at: SimTime,
    },
    Reparented {
        child: EntityId,
        old_parent: EntityId,
        new_parent: EntityId,
        at: SimTime,
    },
    CascadedDespawn {
        entity: EntityId,
        cascade_count: usize,
        at: SimTime,
    },
}

/// Error conditions for linked entity operations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LinkedEntityError {
    StaleEntityHandle(EntityId),
    CycleDetected {
        parent: EntityId,
        child: EntityId,
    },
    DuplicateRelationship {
        parent: EntityId,
        child: EntityId,
    },
    CapacityExceeded {
        max_capacity: usize,
        requested: usize,
    },
    InvalidReparenting {
        child: EntityId,
        new_parent: EntityId,
    },
    EntityNotFound(EntityId),
    StableIdCollision(ModelEntityId),
    ParentNotFound(EntityId),
    ChildNotFound(EntityId),
    UnsupportedSchemaVersion(u16),
    MalformedTelemetry(String),
    MissingTelemetryField(&'static str),
}

impl fmt::Display for LinkedEntityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StaleEntityHandle(e) => write!(f, "stale entity handle: {:?}", e),
            Self::CycleDetected { parent, child } => {
                write!(
                    f,
                    "cycle detected: child {:?} is an ancestor of {:?}",
                    child, parent
                )
            }
            Self::DuplicateRelationship { parent, child } => {
                write!(
                    f,
                    "duplicate relationship between {:?} and {:?}",
                    parent, child
                )
            }
            Self::CapacityExceeded {
                max_capacity,
                requested,
            } => write!(
                f,
                "capacity exceeded: max {}, requested {}",
                max_capacity, requested
            ),
            Self::InvalidReparenting { child, new_parent } => {
                write!(
                    f,
                    "invalid reparenting: child {:?}, new parent {:?}",
                    child, new_parent
                )
            }
            Self::EntityNotFound(e) => write!(f, "entity not found: {:?}", e),
            Self::StableIdCollision(id) => write!(f, "stable ID collision: {}", id),
            Self::ParentNotFound(e) => write!(f, "parent not found: {:?}", e),
            Self::ChildNotFound(e) => write!(f, "child not found: {:?}", e),
            Self::UnsupportedSchemaVersion(v) => write!(f, "unsupported schema version: {}", v),
            Self::MalformedTelemetry(msg) => write!(f, "malformed telemetry: {}", msg),
            Self::MissingTelemetryField(field) => write!(f, "missing telemetry field: {}", field),
        }
    }
}

impl std::error::Error for LinkedEntityError {}

/// Telemetry record capturing an edge or lifecycle transition for Arrow/Parquet export.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelationshipTelemetryRecord {
    pub schema_version: u16,
    pub run_id: String,
    pub event_seq: u64,
    pub time_ticks: u128,
    pub parent_index: u64,
    pub parent_generation: u32,
    pub child_index: u64,
    pub child_generation: u32,
    pub parent_model_id: Option<u64>,
    pub child_model_id: Option<u64>,
    pub kind: u32,
    pub policy: u8,
    pub lifecycle_event: String,
}

impl RelationshipTelemetryRecord {
    pub fn validate(&self) -> Result<(), LinkedEntityError> {
        if self.schema_version != RELATIONSHIP_SCHEMA_VERSION {
            return Err(LinkedEntityError::UnsupportedSchemaVersion(
                self.schema_version,
            ));
        }
        if self.parent_index == self.child_index && self.parent_generation == self.child_generation
        {
            return Err(LinkedEntityError::CycleDetected {
                parent: EntityId::new(self.parent_index, self.parent_generation),
                child: EntityId::new(self.child_index, self.child_generation),
            });
        }
        Ok(())
    }

    pub fn to_ndjson_line(&self) -> String {
        format!(
            "{{\"schema_version\":{},\"run_id\":\"{}\",\"event_seq\":{},\"time_ticks\":{},\"parent_index\":{},\"parent_generation\":{},\"child_index\":{},\"child_generation\":{},\"parent_model_id\":{},\"child_model_id\":{},\"kind\":{},\"policy\":{},\"lifecycle_event\":\"{}\"}}",
            self.schema_version,
            self.run_id,
            self.event_seq,
            self.time_ticks,
            self.parent_index,
            self.parent_generation,
            self.child_index,
            self.child_generation,
            self.parent_model_id.map(|v| v.to_string()).unwrap_or_else(|| "null".to_string()),
            self.child_model_id.map(|v| v.to_string()).unwrap_or_else(|| "null".to_string()),
            self.kind,
            self.policy,
            self.lifecycle_event
        )
    }

    pub fn parse_ndjson_line(line: &str) -> Result<Self, LinkedEntityError> {
        let trimmed = line.trim();
        if !trimmed.starts_with('{') || !trimmed.ends_with('}') {
            return Err(LinkedEntityError::MalformedTelemetry(
                "invalid JSON object".into(),
            ));
        }

        fn extract_str(content: &str, key: &str) -> Result<String, LinkedEntityError> {
            let pattern = format!("\"{}\":\"", key);
            let start = content
                .find(&pattern)
                .ok_or(LinkedEntityError::MissingTelemetryField("key not found"))?
                + pattern.len();
            let end = content[start..]
                .find('"')
                .ok_or(LinkedEntityError::MalformedTelemetry(
                    "unterminated string".into(),
                ))?
                + start;
            Ok(content[start..end].to_string())
        }

        fn extract_u64(content: &str, key: &str) -> Result<u64, LinkedEntityError> {
            let pattern = format!("\"{}\":", key);
            let start = content
                .find(&pattern)
                .ok_or(LinkedEntityError::MissingTelemetryField("key not found"))?
                + pattern.len();
            let remaining = &content[start..];
            let end = remaining.find([',', '}']).unwrap_or(remaining.len());
            let val_str = remaining[..end].trim();
            val_str.parse::<u64>().map_err(|_| {
                LinkedEntityError::MalformedTelemetry(format!("invalid u64 for {}", key))
            })
        }

        fn extract_opt_u64(content: &str, key: &str) -> Result<Option<u64>, LinkedEntityError> {
            let pattern = format!("\"{}\":", key);
            let start = content
                .find(&pattern)
                .ok_or(LinkedEntityError::MissingTelemetryField("key not found"))?
                + pattern.len();
            let remaining = &content[start..];
            let end = remaining.find([',', '}']).unwrap_or(remaining.len());
            let val_str = remaining[..end].trim();
            if val_str == "null" {
                Ok(None)
            } else {
                val_str.parse::<u64>().map(Some).map_err(|_| {
                    LinkedEntityError::MalformedTelemetry(format!("invalid opt u64 for {}", key))
                })
            }
        }

        fn extract_u128(content: &str, key: &str) -> Result<u128, LinkedEntityError> {
            let pattern = format!("\"{}\":", key);
            let start = content
                .find(&pattern)
                .ok_or(LinkedEntityError::MissingTelemetryField("key not found"))?
                + pattern.len();
            let remaining = &content[start..];
            let end = remaining.find([',', '}']).unwrap_or(remaining.len());
            let val_str = remaining[..end].trim();
            val_str.parse::<u128>().map_err(|_| {
                LinkedEntityError::MalformedTelemetry(format!("invalid u128 for {}", key))
            })
        }

        let inner = &trimmed[1..trimmed.len() - 1];
        let schema_version = extract_u64(inner, "schema_version")? as u16;
        let run_id = extract_str(inner, "run_id")?;
        let event_seq = extract_u64(inner, "event_seq")?;
        let time_ticks = extract_u128(inner, "time_ticks")?;
        let parent_index = extract_u64(inner, "parent_index")?;
        let parent_generation = extract_u64(inner, "parent_generation")? as u32;
        let child_index = extract_u64(inner, "child_index")?;
        let child_generation = extract_u64(inner, "child_generation")? as u32;
        let parent_model_id = extract_opt_u64(inner, "parent_model_id")?;
        let child_model_id = extract_opt_u64(inner, "child_model_id")?;
        let kind = extract_u64(inner, "kind")? as u32;
        let policy = extract_u64(inner, "policy")? as u8;
        let lifecycle_event = extract_str(inner, "lifecycle_event")?;

        let record = Self {
            schema_version,
            run_id,
            event_seq,
            time_ticks,
            parent_index,
            parent_generation,
            child_index,
            child_generation,
            parent_model_id,
            child_model_id,
            kind,
            policy,
            lifecycle_event,
        };

        record.validate()?;
        Ok(record)
    }
}

/// Batch container for relationship telemetry records with schema enforcement.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RelationshipTelemetryBatch {
    records: Vec<RelationshipTelemetryRecord>,
}

impl RelationshipTelemetryBatch {
    pub fn new(records: Vec<RelationshipTelemetryRecord>) -> Result<Self, LinkedEntityError> {
        let batch = Self { records };
        batch.validate()?;
        Ok(batch)
    }

    pub fn records(&self) -> &[RelationshipTelemetryRecord] {
        &self.records
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn validate(&self) -> Result<(), LinkedEntityError> {
        let mut seen_attached = std::collections::HashSet::new();

        for record in &self.records {
            record.validate()?;
            if record.lifecycle_event == "attached" {
                let edge = (
                    record.parent_index,
                    record.parent_generation,
                    record.child_index,
                    record.child_generation,
                    record.kind,
                );
                if !seen_attached.insert(edge) {
                    return Err(LinkedEntityError::DuplicateRelationship {
                        parent: EntityId::new(record.parent_index, record.parent_generation),
                        child: EntityId::new(record.child_index, record.child_generation),
                    });
                }
            }
        }
        Ok(())
    }

    pub fn to_ndjson(&self) -> String {
        let mut lines = Vec::with_capacity(self.records.len());
        for rec in &self.records {
            lines.push(rec.to_ndjson_line());
        }
        lines.join("\n")
    }

    pub fn from_ndjson(data: &str) -> Result<Self, LinkedEntityError> {
        let mut records = Vec::new();
        for line in data.lines() {
            let line = line.trim();
            if !line.is_empty() {
                records.push(RelationshipTelemetryRecord::parse_ndjson_line(line)?);
            }
        }
        Self::new(records)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.to_ndjson().into_bytes()
    }

    pub fn from_bytes(payload: &[u8]) -> Result<Self, LinkedEntityError> {
        let text = std::str::from_utf8(payload)
            .map_err(|_| LinkedEntityError::MalformedTelemetry("invalid UTF-8".into()))?;
        Self::from_ndjson(text)
    }
}
