use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
    #[serde(deserialize_with = "deserialize_kind_order")]
    pub expected_kind_order: Vec<u32>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum KindOrderRepr {
    List(Vec<u32>),
    String(String),
}

fn deserialize_kind_order<'de, D>(deserializer: D) -> Result<Vec<u32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let repr = KindOrderRepr::deserialize(deserializer)?;
    match repr {
        KindOrderRepr::List(list) => {
            if list.is_empty() {
                return Err(serde::de::Error::custom(
                    "expected_kind_order cannot be empty",
                ));
            }
            Ok(list)
        }
        KindOrderRepr::String(s) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                return Err(serde::de::Error::custom(
                    "expected_kind_order cannot be empty",
                ));
            }
            trimmed
                .split(',')
                .map(|item| {
                    item.trim().parse::<u32>().map_err(|e| {
                        serde::de::Error::custom(format!("invalid u32 in expected_kind_order: {e}"))
                    })
                })
                .collect()
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SeedManifest {
    pub schema_version: String,
    pub scenario_id: String,
    pub base_seed: u64,
    pub fixture_id: String,
    #[serde(default)]
    pub streams: BTreeMap<String, u64>,
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
    let text = fs::read_to_string(path).map_err(|source| ScenarioError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    toml::from_str(&text).map_err(|e| map_toml_error(e, &text))
}

pub fn load_seed_manifest(path: &Path) -> Result<SeedManifest, ScenarioError> {
    let text = fs::read_to_string(path).map_err(|source| ScenarioError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    toml::from_str(&text).map_err(|e| map_toml_error(e, &text))
}

fn map_toml_error(err: toml::de::Error, text: &str) -> ScenarioError {
    let msg = err.message();
    if let Some(rest) = msg.strip_prefix("missing field `") {
        if let Some(field) = rest.strip_suffix('`') {
            let static_field = match field {
                "schema_version" => "schema_version",
                "scenario_id" => "scenario_id",
                "model_id" => "model_id",
                "fixture_id" => "fixture_id",
                "fixture_path" => "fixture_path",
                "base_seed" => "base_seed",
                "replications" => "replications",
                "max_events" => "max_events",
                "artifact_root" => "artifact_root",
                "resume_checkpoint_every_events" => "resume_checkpoint_every_events",
                "expected_kind_order" => "expected_kind_order",
                other => Box::leak(other.to_string().into_boxed_str()),
            };
            return ScenarioError::MissingField(static_field);
        }
    }
    if msg.contains("expected_kind_order") {
        return ScenarioError::InvalidField {
            field: "expected_kind_order",
            value: msg.to_string(),
        };
    }
    if let Some(span) = err.span() {
        let prefix = &text[..span.start.min(text.len())];
        let line_start = prefix.rfind('\n').map(|i| i + 1).unwrap_or(0);
        let line = &text[line_start..];
        let line = line.lines().next().unwrap_or("");
        if let Some((lhs, _)) = line.split_once('=') {
            let key = lhs.trim();
            let static_field = match key {
                "schema_version" => Some("schema_version"),
                "scenario_id" => Some("scenario_id"),
                "model_id" => Some("model_id"),
                "fixture_id" => Some("fixture_id"),
                "fixture_path" => Some("fixture_path"),
                "base_seed" => Some("base_seed"),
                "replications" => Some("replications"),
                "max_events" => Some("max_events"),
                "artifact_root" => Some("artifact_root"),
                "resume_checkpoint_every_events" => Some("resume_checkpoint_every_events"),
                "expected_kind_order" => Some("expected_kind_order"),
                _ => None,
            };
            if let Some(field) = static_field {
                return ScenarioError::InvalidField {
                    field,
                    value: msg.to_string(),
                };
            }
        }
    }
    let err_str = err.to_string();
    for field in [
        "schema_version",
        "scenario_id",
        "model_id",
        "fixture_id",
        "fixture_path",
        "base_seed",
        "replications",
        "max_events",
        "artifact_root",
        "resume_checkpoint_every_events",
        "expected_kind_order",
    ] {
        if err_str.contains(&format!("{} =", field)) || err_str.contains(&format!("{}=", field)) {
            return ScenarioError::InvalidField {
                field,
                value: msg.to_string(),
            };
        }
    }
    ScenarioError::InvalidField {
        field: "toml",
        value: err.to_string(),
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_MANIFEST_ID: AtomicU64 = AtomicU64::new(0);

    struct TestManifestFile(PathBuf);

    impl TestManifestFile {
        fn new(contents: &str) -> Self {
            let id = NEXT_MANIFEST_ID.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "kairos-manifest-test-{}-{id}.toml",
                std::process::id()
            ));
            fs::write(&path, contents).expect("write temporary manifest");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestManifestFile {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }

    #[test]
    fn load_scenario_parses_manifest_fields() {
        let file = TestManifestFile::new(
            "schema_version = 'kairoecs.scenario.v1'\n\
             scenario_id = 'test_scenario'\n\
             model_id = 'test_model'\n\
             fixture_id = 'test_fixture'\n\
             fixture_path = '/tmp/fixture'\n\
             base_seed = 42\n\
             replications = 3\n\
             max_events = 1000\n\
             artifact_root = '/tmp/artifacts'\n\
             resume_checkpoint_every_events = 500\n\
             expected_kind_order = [1, 2, 3]\n",
        );

        let scenario = load_scenario(file.path()).unwrap();

        assert_eq!(scenario.schema_version, "kairoecs.scenario.v1");
        assert_eq!(scenario.scenario_id, "test_scenario");
        assert_eq!(scenario.model_id, "test_model");
        assert_eq!(scenario.fixture_id, "test_fixture");
        assert_eq!(scenario.fixture_path, PathBuf::from("/tmp/fixture"));
        assert_eq!(scenario.base_seed, 42);
        assert_eq!(scenario.replications, 3);
        assert_eq!(scenario.max_events, 1000);
        assert_eq!(scenario.artifact_root, PathBuf::from("/tmp/artifacts"));
        assert_eq!(scenario.resume_checkpoint_every_events, 500);
        assert_eq!(scenario.expected_kind_order, vec![1, 2, 3]);
    }

    #[test]
    fn load_scenario_reports_missing_field() {
        let file = TestManifestFile::new(
            "schema_version = 'kairoecs.scenario.v1'\nscenario_id = 'test_scenario'\n",
        );

        assert!(matches!(
            load_scenario(file.path()),
            Err(ScenarioError::MissingField("model_id"))
        ));
    }

    #[test]
    fn load_scenario_reports_invalid_numeric_value() {
        let file = TestManifestFile::new(
            "schema_version = 'kairoecs.scenario.v1'\n\
             scenario_id = 'test_scenario'\n\
             model_id = 'test_model'\n\
             fixture_id = 'test_fixture'\n\
             fixture_path = '/tmp/fixture'\n\
             base_seed = 'not_a_number'\n\
             replications = 3\n\
             max_events = 1000\n\
             artifact_root = '/tmp/artifacts'\n\
             resume_checkpoint_every_events = 500\n\
             expected_kind_order = [1, 2, 3]\n",
        );

        assert!(matches!(
            load_scenario(file.path()),
            Err(ScenarioError::InvalidField {
                field: "base_seed",
                ..
            })
        ));
    }

    #[test]
    fn load_scenario_reports_invalid_kind_order() {
        let file = TestManifestFile::new(
            "schema_version = 'kairoecs.scenario.v1'\n\
             scenario_id = 'test_scenario'\n\
             model_id = 'test_model'\n\
             fixture_id = 'test_fixture'\n\
             fixture_path = '/tmp/fixture'\n\
             base_seed = 42\n\
             replications = 3\n\
             max_events = 1000\n\
             artifact_root = '/tmp/artifacts'\n\
             resume_checkpoint_every_events = 500\n\
             expected_kind_order = '1, invalid, 3'\n",
        );

        assert!(matches!(
            load_scenario(file.path()),
            Err(ScenarioError::InvalidField {
                field: "expected_kind_order",
                ..
            })
        ));
    }

    #[test]
    fn load_seed_manifest_parses_manifest_fields() {
        let file = TestManifestFile::new(
            "schema_version = 'kairoecs.seed.v1'\n\
             scenario_id = 'test_scenario'\n\
             base_seed = 42\n\
             fixture_id = 'test_fixture'\n\
             \n\
             [streams]\n\
             stream_a = 100\n\
             stream_b = 200\n",
        );

        let seed = load_seed_manifest(file.path()).unwrap();

        assert_eq!(seed.schema_version, "kairoecs.seed.v1");
        assert_eq!(seed.scenario_id, "test_scenario");
        assert_eq!(seed.base_seed, 42);
        assert_eq!(seed.fixture_id, "test_fixture");
        assert_eq!(seed.streams.get("stream_a"), Some(&100));
        assert_eq!(seed.streams.get("stream_b"), Some(&200));
    }

    #[test]
    fn load_seed_manifest_reports_missing_field() {
        let file = TestManifestFile::new(
            "schema_version = 'kairoecs.seed.v1'\n\
             scenario_id = 'test_scenario'\n\
             fixture_id = 'test_fixture'\n",
        );

        assert!(matches!(
            load_seed_manifest(file.path()),
            Err(ScenarioError::MissingField("base_seed"))
        ));
    }

    #[test]
    fn load_seed_manifest_reports_invalid_numeric_value() {
        let file = TestManifestFile::new(
            "schema_version = 'kairoecs.seed.v1'\n\
             scenario_id = 'test_scenario'\n\
             base_seed = 'not_a_number'\n\
             fixture_id = 'test_fixture'\n",
        );

        assert!(matches!(
            load_seed_manifest(file.path()),
            Err(ScenarioError::InvalidField {
                field: "base_seed",
                ..
            })
        ));
    }

    #[test]
    fn toml_sections_comments_and_escapes() {
        let file = TestManifestFile::new(
            "# Leading scenario comment\n\
             schema_version = \"kairoecs.scenario.v1\" # inline comment\n\
             scenario_id = \"scenario-\\\"escaped\\\"\\n\"\n\
             model_id = \"model_id_val\"\n\
             fixture_id = \"fixture_val\"\n\
             fixture_path = \"/path/with/spaces and \\\"quotes\\\"\"\n\
             base_seed = 100\n\
             replications = 5\n\
             max_events = 50\n\
             artifact_root = \"/tmp/artifacts\"\n\
             resume_checkpoint_every_events = 25\n\
             # Array with comments\n\
             expected_kind_order = [\n\
                 1, # first\n\
                 2, # second\n\
                 3  # third\n\
             ]\n",
        );

        let scenario =
            load_scenario(file.path()).expect("parse valid toml with comments and escapes");
        assert_eq!(scenario.scenario_id, "scenario-\"escaped\"\n");
        assert_eq!(scenario.expected_kind_order, vec![1, 2, 3]);
    }

    #[test]
    fn toml_legacy_comma_delimited_string_compatibility() {
        let file = TestManifestFile::new(
            "schema_version = \"kairoecs.scenario.v1\"\n\
             scenario_id = \"scen_compat\"\n\
             model_id = \"m1\"\n\
             fixture_id = \"f1\"\n\
             fixture_path = \"/tmp/f\"\n\
             base_seed = 1\n\
             replications = 1\n\
             max_events = 1\n\
             artifact_root = \"/tmp\"\n\
             resume_checkpoint_every_events = 1\n\
             expected_kind_order = \"1, 2, 4, 3\"\n",
        );

        let scenario = load_scenario(file.path()).expect("parse comma-delimited string");
        assert_eq!(scenario.expected_kind_order, vec![1, 2, 4, 3]);
    }

    #[test]
    fn toml_rejects_unknown_fields_in_scenario() {
        let file = TestManifestFile::new(
            "schema_version = \"kairoecs.scenario.v1\"\n\
             scenario_id = \"scen1\"\n\
             model_id = \"m1\"\n\
             fixture_id = \"f1\"\n\
             fixture_path = \"/tmp\"\n\
             base_seed = 1\n\
             replications = 1\n\
             max_events = 1\n\
             artifact_root = \"/tmp\"\n\
             resume_checkpoint_every_events = 1\n\
             expected_kind_order = [1]\n\
             unexpected_field = true\n",
        );

        assert!(matches!(
            load_scenario(file.path()),
            Err(ScenarioError::InvalidField { .. })
        ));
    }

    #[test]
    fn toml_rejects_duplicate_keys() {
        let file = TestManifestFile::new(
            "schema_version = \"kairoecs.scenario.v1\"\n\
             scenario_id = \"first\"\n\
             scenario_id = \"duplicate\"\n",
        );

        assert!(matches!(
            load_scenario(file.path()),
            Err(ScenarioError::InvalidField { .. })
        ));
    }

    fn valid_scenario() -> ScenarioManifest {
        ScenarioManifest {
            schema_version: "kairoecs.scenario.v1".to_string(),
            scenario_id: "test_scenario".to_string(),
            model_id: "test_model".to_string(),
            fixture_id: "test_fixture".to_string(),
            fixture_path: PathBuf::from("."),
            base_seed: 42,
            replications: 10,
            max_events: 1000,
            artifact_root: PathBuf::from("."),
            resume_checkpoint_every_events: 100,
            expected_kind_order: vec![1, 2, 3],
        }
    }

    fn valid_seed() -> SeedManifest {
        SeedManifest {
            schema_version: "kairoecs.seed.v1".to_string(),
            scenario_id: "test_scenario".to_string(),
            base_seed: 42,
            fixture_id: "test_fixture".to_string(),
            streams: BTreeMap::new(),
        }
    }

    #[test]
    fn test_validate_scenario_and_seed_success() {
        let scenario = valid_scenario();
        let seed = valid_seed();
        assert!(validate_scenario_and_seed(&scenario, &seed).is_ok());
    }

    #[test]
    fn test_validate_scenario_invalid_schema() {
        let mut scenario = valid_scenario();
        let seed = valid_seed();
        scenario.schema_version = "invalid.v1".to_string();
        let err = validate_scenario_and_seed(&scenario, &seed).unwrap_err();
        assert!(matches!(
            err,
            ScenarioError::InvalidField {
                field: "schema_version",
                ..
            }
        ));
    }

    #[test]
    fn test_validate_seed_invalid_schema() {
        let scenario = valid_scenario();
        let mut seed = valid_seed();
        seed.schema_version = "invalid.v1".to_string();
        let err = validate_scenario_and_seed(&scenario, &seed).unwrap_err();
        assert!(matches!(
            err,
            ScenarioError::InvalidField {
                field: "schema_version",
                ..
            }
        ));
    }

    #[test]
    fn test_validate_mismatch_scenario_id() {
        let mut scenario = valid_scenario();
        let seed = valid_seed();
        scenario.scenario_id = "other_scenario".to_string();
        let err = validate_scenario_and_seed(&scenario, &seed).unwrap_err();
        assert!(
            matches!(err, ScenarioError::Mismatch(msg) if msg.contains("scenario_id mismatch"))
        );
    }

    #[test]
    fn test_validate_mismatch_base_seed() {
        let mut scenario = valid_scenario();
        let seed = valid_seed();
        scenario.base_seed = 99;
        let err = validate_scenario_and_seed(&scenario, &seed).unwrap_err();
        assert!(matches!(err, ScenarioError::Mismatch(msg) if msg.contains("base_seed mismatch")));
    }

    #[test]
    fn test_validate_mismatch_fixture_id() {
        let mut scenario = valid_scenario();
        let seed = valid_seed();
        scenario.fixture_id = "other_fixture".to_string();
        let err = validate_scenario_and_seed(&scenario, &seed).unwrap_err();
        assert!(matches!(err, ScenarioError::Mismatch(msg) if msg.contains("fixture_id mismatch")));
    }

    #[test]
    fn test_validate_zero_replications() {
        let mut scenario = valid_scenario();
        let seed = valid_seed();
        scenario.replications = 0;
        let err = validate_scenario_and_seed(&scenario, &seed).unwrap_err();
        assert!(matches!(
            err,
            ScenarioError::InvalidField {
                field: "replications",
                ..
            }
        ));
    }

    #[test]
    fn test_validate_zero_max_events() {
        let mut scenario = valid_scenario();
        let seed = valid_seed();
        scenario.max_events = 0;
        let err = validate_scenario_and_seed(&scenario, &seed).unwrap_err();
        assert!(matches!(
            err,
            ScenarioError::InvalidField {
                field: "max_events",
                ..
            }
        ));
    }

    #[test]
    fn test_validate_empty_expected_kind_order() {
        let mut scenario = valid_scenario();
        let seed = valid_seed();
        scenario.expected_kind_order = vec![];
        let err = validate_scenario_and_seed(&scenario, &seed).unwrap_err();
        assert!(matches!(
            err,
            ScenarioError::MissingField("expected_kind_order")
        ));
    }

    #[test]
    fn test_validate_missing_fixture_path() {
        let mut scenario = valid_scenario();
        let seed = valid_seed();
        scenario.fixture_path = std::env::temp_dir()
            .join(format!("kairos-missing-fixture-{}", std::process::id()))
            .join("fixture");
        let err = validate_scenario_and_seed(&scenario, &seed).unwrap_err();
        assert!(
            matches!(err, ScenarioError::Mismatch(msg) if msg.contains("fixture_path does not exist"))
        );
    }
}
