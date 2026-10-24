use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::path::PathBuf;

pub const RUN_MANIFEST_SCHEMA_V1: &str = "kairoecs.run-manifest.v1";
pub const RESUMABILITY_PLAN_SCHEMA_V1: &str = "kairoecs.resumability-plan.v1";
pub const CHECKPOINT_MANIFEST_SCHEMA_V1: &str = "kairoecs.checkpoint-manifest.v1";
pub const RESUME_REQUEST_SCHEMA_V1: &str = "kairoecs.resume-request.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContractValidationError {
    MissingField(&'static str),
    InvalidSchemaVersion {
        expected: &'static str,
        found: String,
    },
    ZeroValue(&'static str),
    EmptyField(&'static str),
}

impl Display for ContractValidationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingField(field) => write!(f, "missing required contract field `{field}`"),
            Self::InvalidSchemaVersion { expected, found } => {
                write!(
                    f,
                    "invalid schema version: expected `{expected}`, found `{found}`"
                )
            }
            Self::ZeroValue(field) => write!(f, "field `{field}` must be non-zero"),
            Self::EmptyField(field) => write!(f, "field `{field}` cannot be empty"),
        }
    }
}

impl std::error::Error for ContractValidationError {}

/// Typed DTO for the execution run manifest.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RunManifestDto {
    pub schema_version: String,
    pub scenario_id: String,
    pub fixture_id: String,
    pub base_seed: u64,
    pub replications: u32,
    pub artifact_root: PathBuf,
}

impl RunManifestDto {
    pub fn new(
        scenario_id: impl Into<String>,
        fixture_id: impl Into<String>,
        base_seed: u64,
        replications: u32,
        artifact_root: impl Into<PathBuf>,
    ) -> Self {
        Self {
            schema_version: RUN_MANIFEST_SCHEMA_V1.to_string(),
            scenario_id: scenario_id.into(),
            fixture_id: fixture_id.into(),
            base_seed,
            replications,
            artifact_root: artifact_root.into(),
        }
    }

    pub fn validate(&self) -> Result<(), ContractValidationError> {
        if self.schema_version != RUN_MANIFEST_SCHEMA_V1 {
            return Err(ContractValidationError::InvalidSchemaVersion {
                expected: RUN_MANIFEST_SCHEMA_V1,
                found: self.schema_version.clone(),
            });
        }
        if self.scenario_id.is_empty() {
            return Err(ContractValidationError::EmptyField("scenario_id"));
        }
        if self.fixture_id.is_empty() {
            return Err(ContractValidationError::EmptyField("fixture_id"));
        }
        if self.replications == 0 {
            return Err(ContractValidationError::ZeroValue("replications"));
        }
        Ok(())
    }
}

/// Typed DTO for execution replay summary output.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SummaryDto {
    pub scenario_id: String,
    pub event_count: u64,
    pub observed_kind_order: Vec<u32>,
    pub summary_hash: String,
}

impl SummaryDto {
    pub fn new(
        scenario_id: impl Into<String>,
        event_count: u64,
        observed_kind_order: Vec<u32>,
        summary_hash: impl Into<String>,
    ) -> Self {
        Self {
            scenario_id: scenario_id.into(),
            event_count,
            observed_kind_order,
            summary_hash: summary_hash.into(),
        }
    }

    pub fn validate(&self) -> Result<(), ContractValidationError> {
        if self.scenario_id.is_empty() {
            return Err(ContractValidationError::EmptyField("scenario_id"));
        }
        if self.summary_hash.is_empty() {
            return Err(ContractValidationError::EmptyField("summary_hash"));
        }
        Ok(())
    }
}

/// Typed DTO for replay comparison validation results.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReplayComparisonDto {
    pub fixture_id: String,
    pub status: String,
    pub comparison_basis: String,
    pub summary_hash: String,
}

impl ReplayComparisonDto {
    pub fn new_matched(fixture_id: impl Into<String>, summary_hash: impl Into<String>) -> Self {
        Self {
            fixture_id: fixture_id.into(),
            status: "matched".to_string(),
            comparison_basis: "expected_kind_order".to_string(),
            summary_hash: summary_hash.into(),
        }
    }

    pub fn validate(&self) -> Result<(), ContractValidationError> {
        if self.fixture_id.is_empty() {
            return Err(ContractValidationError::EmptyField("fixture_id"));
        }
        if self.status.is_empty() {
            return Err(ContractValidationError::EmptyField("status"));
        }
        if self.comparison_basis.is_empty() {
            return Err(ContractValidationError::EmptyField("comparison_basis"));
        }
        if self.summary_hash.is_empty() {
            return Err(ContractValidationError::EmptyField("summary_hash"));
        }
        Ok(())
    }
}

/// Typed DTO for execution resumability plan.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ResumabilityPlanDto {
    pub schema_version: String,
    pub scenario_id: String,
    pub checkpoint_every_events: u64,
    pub resume_requires: Vec<String>,
}

impl ResumabilityPlanDto {
    pub fn default_for_scenario(
        scenario_id: impl Into<String>,
        checkpoint_every_events: u64,
    ) -> Self {
        Self {
            schema_version: RESUMABILITY_PLAN_SCHEMA_V1.to_string(),
            scenario_id: scenario_id.into(),
            checkpoint_every_events,
            resume_requires: vec![
                "scenario manifest".to_string(),
                "seed manifest".to_string(),
                "last completed event index".to_string(),
                "summary hash comparison".to_string(),
            ],
        }
    }

    pub fn validate(&self) -> Result<(), ContractValidationError> {
        if self.schema_version != RESUMABILITY_PLAN_SCHEMA_V1 {
            return Err(ContractValidationError::InvalidSchemaVersion {
                expected: RESUMABILITY_PLAN_SCHEMA_V1,
                found: self.schema_version.clone(),
            });
        }
        if self.scenario_id.is_empty() {
            return Err(ContractValidationError::EmptyField("scenario_id"));
        }
        if self.checkpoint_every_events == 0 {
            return Err(ContractValidationError::ZeroValue(
                "checkpoint_every_events",
            ));
        }
        Ok(())
    }
}

/// Typed DTO for checkpoint manifest output.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CheckpointManifestDto {
    pub schema_version: String,
    pub last_completed_tick: String,
    pub state: String,
}

impl CheckpointManifestDto {
    pub fn new(last_completed_tick: impl Into<String>, state: impl Into<String>) -> Self {
        Self {
            schema_version: CHECKPOINT_MANIFEST_SCHEMA_V1.to_string(),
            last_completed_tick: last_completed_tick.into(),
            state: state.into(),
        }
    }

    pub fn validate(&self) -> Result<(), ContractValidationError> {
        if self.schema_version != CHECKPOINT_MANIFEST_SCHEMA_V1 {
            return Err(ContractValidationError::InvalidSchemaVersion {
                expected: CHECKPOINT_MANIFEST_SCHEMA_V1,
                found: self.schema_version.clone(),
            });
        }
        if self.state.is_empty() {
            return Err(ContractValidationError::EmptyField("state"));
        }
        Ok(())
    }
}

/// Typed DTO for resume request inputs.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ResumeRequestDto {
    pub schema_version: String,
    pub checkpoint: String,
    pub status: String,
}

impl ResumeRequestDto {
    pub fn new(checkpoint: impl Into<String>, status: impl Into<String>) -> Self {
        Self {
            schema_version: RESUME_REQUEST_SCHEMA_V1.to_string(),
            checkpoint: checkpoint.into(),
            status: status.into(),
        }
    }

    pub fn validate(&self) -> Result<(), ContractValidationError> {
        if self.schema_version != RESUME_REQUEST_SCHEMA_V1 {
            return Err(ContractValidationError::InvalidSchemaVersion {
                expected: RESUME_REQUEST_SCHEMA_V1,
                found: self.schema_version.clone(),
            });
        }
        if self.checkpoint.is_empty() {
            return Err(ContractValidationError::EmptyField("checkpoint"));
        }
        if self.status.is_empty() {
            return Err(ContractValidationError::EmptyField("status"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_manifest_dto_roundtrip_and_validation() {
        let original = RunManifestDto::new(
            "scen-01",
            "fix-01",
            u64::MAX,
            u32::MAX,
            PathBuf::from("/artifacts/run"),
        );
        assert!(original.validate().is_ok());

        let serialized = serde_json::to_string_pretty(&original).expect("serialize");
        let deserialized: RunManifestDto = serde_json::from_str(&serialized).expect("deserialize");
        assert_eq!(original, deserialized);
        assert_eq!(deserialized.base_seed, u64::MAX);
        assert_eq!(deserialized.replications, u32::MAX);
    }

    #[test]
    fn run_manifest_dto_rejects_unknown_fields() {
        let json = r#"{
            "schema_version": "kairoecs.run-manifest.v1",
            "scenario_id": "s1",
            "fixture_id": "f1",
            "base_seed": 42,
            "replications": 1,
            "artifact_root": "/tmp",
            "unexpected_injected_field": true
        }"#;
        let result: Result<RunManifestDto, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn run_manifest_dto_rejects_invalid_schema_version() {
        let mut dto = RunManifestDto::new("s1", "f1", 42, 1, PathBuf::from("/tmp"));
        dto.schema_version = "kairoecs.run-manifest.v2-unsupported".to_string();
        assert!(matches!(
            dto.validate(),
            Err(ContractValidationError::InvalidSchemaVersion { .. })
        ));
    }

    #[test]
    fn summary_dto_roundtrip_full_ranges() {
        let original = SummaryDto::new(
            "scenario-alpha",
            u64::MAX,
            vec![0, 1, 42, u32::MAX],
            "hash-1234567890abcdef",
        );
        assert!(original.validate().is_ok());

        let serialized = serde_json::to_string(&original).expect("serialize");
        let deserialized: SummaryDto = serde_json::from_str(&serialized).expect("deserialize");
        assert_eq!(original, deserialized);
        assert_eq!(deserialized.event_count, u64::MAX);
        assert_eq!(deserialized.observed_kind_order, vec![0, 1, 42, u32::MAX]);
    }

    #[test]
    fn resumability_plan_dto_roundtrip() {
        let original = ResumabilityPlanDto::default_for_scenario("scen-42", u64::MAX);
        assert!(original.validate().is_ok());

        let serialized = serde_json::to_string(&original).expect("serialize");
        let deserialized: ResumabilityPlanDto =
            serde_json::from_str(&serialized).expect("deserialize");
        assert_eq!(original, deserialized);
        assert_eq!(deserialized.checkpoint_every_events, u64::MAX);
    }

    #[test]
    fn checkpoint_manifest_and_resume_request_roundtrips() {
        let checkpoint = CheckpointManifestDto::new("18446744073709551615", "interrupted");
        assert!(checkpoint.validate().is_ok());
        let ser = serde_json::to_string(&checkpoint).expect("serialize");
        let de: CheckpointManifestDto = serde_json::from_str(&ser).expect("deserialize");
        assert_eq!(checkpoint, de);

        let resume = ResumeRequestDto::new("/checkpoints/18446744073709551615.json", "accepted");
        assert!(resume.validate().is_ok());
        let ser = serde_json::to_string(&resume).expect("serialize");
        let de: ResumeRequestDto = serde_json::from_str(&ser).expect("deserialize");
        assert_eq!(resume, de);
    }
}
