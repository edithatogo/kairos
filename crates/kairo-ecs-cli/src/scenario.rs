use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScenarioManifest {
    pub schema_version: String,
    pub scenario_id: String,
    pub model_id: String,
    pub fixture_id: String,
    pub fixture_path: PathBuf,
    pub base_seed: u64,
    pub replications: u32,
    pub max_events: u64,
    pub artifact_root: PathBuf,
    pub resume_checkpoint_every_events: u64,
    pub expected_kind_order: Vec<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SeedManifest {
    pub schema_version: String,
    pub scenario_id: String,
    pub base_seed: u64,
    pub fixture_id: String,
}

#[derive(Debug)]
pub enum ScenarioError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    MissingField(&'static str),
    InvalidField {
        field: &'static str,
        value: String,
    },
    Mismatch(String),
}

impl Display for ScenarioError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "{}: {}", path.display(), source),
            Self::MissingField(field) => write!(f, "missing required field `{field}`"),
            Self::InvalidField { field, value } => {
                write!(f, "invalid value for `{field}`: {value}")
            }
            Self::Mismatch(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for ScenarioError {}

pub fn load_scenario(path: &Path) -> Result<ScenarioManifest, ScenarioError> {
    let fields = read_fields(path)?;

    Ok(ScenarioManifest {
        schema_version: required_string(&fields, "schema_version")?,
        scenario_id: required_string(&fields, "scenario_id")?,
        model_id: required_string(&fields, "model_id")?,
        fixture_id: required_string(&fields, "fixture_id")?,
        fixture_path: PathBuf::from(required_string(&fields, "fixture_path")?),
        base_seed: required_u64(&fields, "base_seed")?,
        replications: required_u64(&fields, "replications")?
            .try_into()
            .map_err(|_| ScenarioError::InvalidField {
                field: "replications",
                value: required_string(&fields, "replications").unwrap_or_default(),
            })?,
        max_events: required_u64(&fields, "max_events")?,
        artifact_root: PathBuf::from(required_string(&fields, "artifact_root")?),
        resume_checkpoint_every_events: required_u64(&fields, "resume_checkpoint_every_events")?,
        expected_kind_order: required_list(&fields, "expected_kind_order")?,
    })
}

pub fn load_seed_manifest(path: &Path) -> Result<SeedManifest, ScenarioError> {
    let fields = read_fields(path)?;

    Ok(SeedManifest {
        schema_version: required_string(&fields, "schema_version")?,
        scenario_id: required_string(&fields, "scenario_id")?,
        base_seed: required_u64(&fields, "base_seed")?,
        fixture_id: required_string(&fields, "fixture_id")?,
    })
}

pub fn validate_scenario_and_seed(
    scenario: &ScenarioManifest,
    seed: &SeedManifest,
) -> Result<(), ScenarioError> {
    if scenario.schema_version != "kairoecs.scenario.v1" {
        return Err(ScenarioError::InvalidField {
            field: "schema_version",
            value: scenario.schema_version.clone(),
        });
    }

    if seed.schema_version != "kairoecs.seed.v1" {
        return Err(ScenarioError::InvalidField {
            field: "schema_version",
            value: seed.schema_version.clone(),
        });
    }

    if scenario.scenario_id != seed.scenario_id {
        return Err(ScenarioError::Mismatch(format!(
            "scenario_id mismatch: scenario={} seed={}",
            scenario.scenario_id, seed.scenario_id
        )));
    }

    if scenario.base_seed != seed.base_seed {
        return Err(ScenarioError::Mismatch(format!(
            "base_seed mismatch: scenario={} seed={}",
            scenario.base_seed, seed.base_seed
        )));
    }

    if scenario.fixture_id != seed.fixture_id {
        return Err(ScenarioError::Mismatch(format!(
            "fixture_id mismatch: scenario={} seed={}",
            scenario.fixture_id, seed.fixture_id
        )));
    }

    if scenario.replications == 0 {
        return Err(ScenarioError::InvalidField {
            field: "replications",
            value: "0".to_string(),
        });
    }

    if scenario.max_events == 0 {
        return Err(ScenarioError::InvalidField {
            field: "max_events",
            value: "0".to_string(),
        });
    }

    if scenario.expected_kind_order.is_empty() {
        return Err(ScenarioError::MissingField("expected_kind_order"));
    }

    if !scenario.fixture_path.exists() {
        return Err(ScenarioError::Mismatch(format!(
            "fixture_path does not exist: {}",
            scenario.fixture_path.display()
        )));
    }

    Ok(())
}

fn read_fields(path: &Path) -> Result<BTreeMap<String, String>, ScenarioError> {
    let text = fs::read_to_string(path).map_err(|source| ScenarioError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mut fields = BTreeMap::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('[') {
            continue;
        }

        if let Some((key, value)) = trimmed.split_once('=') {
            fields.insert(key.trim().to_string(), unquote(value.trim()));
        }
    }

    Ok(fields)
}

fn required_string(
    fields: &BTreeMap<String, String>,
    field: &'static str,
) -> Result<String, ScenarioError> {
    fields
        .get(field)
        .cloned()
        .filter(|value| !value.is_empty())
        .ok_or(ScenarioError::MissingField(field))
}

fn required_u64(
    fields: &BTreeMap<String, String>,
    field: &'static str,
) -> Result<u64, ScenarioError> {
    let value = required_string(fields, field)?;
    value
        .parse()
        .map_err(|_| ScenarioError::InvalidField { field, value })
}

fn required_list(
    fields: &BTreeMap<String, String>,
    field: &'static str,
) -> Result<Vec<u32>, ScenarioError> {
    let value = required_string(fields, field)?;
    value
        .split(',')
        .map(|item| {
            item.trim()
                .parse()
                .map_err(|_| ScenarioError::InvalidField {
                    field,
                    value: value.clone(),
                })
        })
        .collect()
}

fn unquote(value: &str) -> String {
    value
        .trim_matches('"')
        .trim_matches('\'')
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_seed_manifest_success() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            "schema_version = kairoecs.seed.v1\n\
             scenario_id = test-scenario-1\n\
             base_seed = 42\n\
             fixture_id = test-fixture-1"
        )
        .unwrap();

        let manifest = load_seed_manifest(file.path()).unwrap();
        assert_eq!(manifest.schema_version, "kairoecs.seed.v1");
        assert_eq!(manifest.scenario_id, "test-scenario-1");
        assert_eq!(manifest.base_seed, 42);
        assert_eq!(manifest.fixture_id, "test-fixture-1");
    }

    #[test]
    fn test_load_seed_manifest_missing_field() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            "schema_version = kairoecs.seed.v1\n\
             scenario_id = test-scenario-1\n\
             fixture_id = test-fixture-1"
        )
        .unwrap();

        let result = load_seed_manifest(file.path());
        assert!(matches!(
            result,
            Err(ScenarioError::MissingField("base_seed"))
        ));
    }

    #[test]
    fn test_load_seed_manifest_invalid_field() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            "schema_version = kairoecs.seed.v1\n\
             scenario_id = test-scenario-1\n\
             base_seed = not_a_number\n\
             fixture_id = test-fixture-1"
        )
        .unwrap();

        let result = load_seed_manifest(file.path());
        match result {
            Err(ScenarioError::InvalidField { field, value }) => {
                assert_eq!(field, "base_seed");
                assert_eq!(value, "not_a_number");
            }
            _ => panic!("Expected InvalidField error"),
        }
    }
}
