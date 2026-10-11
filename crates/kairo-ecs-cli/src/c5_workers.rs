//! Bounded, deterministic CPU task execution with durable per-task JSON shards.
//!
//! This is an experimental CLI implementation detail. A task callback owns all
//! simulation state it constructs; this module only transports immutable JSON.

use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

const SHARD_SCHEMA: &str = "kairoecs.c5-task-shard.v1";
const MAX_WORKERS: usize = 64;
const MAX_TASKS: usize = 10_000;
const MAX_TASK_BYTES: usize = 1_048_576;
const MAX_TOTAL_TASK_BYTES: usize = 64 * 1_048_576;
const MAX_SHARD_BYTES: usize = 16 * 1_048_576;
const TEMP_PREFIX: &str = ".c5tmp-";
static TEMP_SEQUENCE: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Execution {
    pub results: Vec<TaskResult>,
    pub complete: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TaskResult {
    pub task_id: String,
    pub result: Value,
}

/// Execute tasks in deterministic task-id order and persist each result before
/// reporting it. `stop_after` limits this call to a prefix of missing tasks.
pub(crate) fn execute<F>(
    tasks: &[Value],
    workers: usize,
    output: &Path,
    binding: &str,
    stop_after: Option<usize>,
    evaluator: F,
) -> Result<Execution, String>
where
    F: Fn(&Value) -> Result<Value, String> + Sync,
{
    if workers == 0 || workers > MAX_WORKERS {
        return Err(format!("workers must be between 1 and {MAX_WORKERS}"));
    }
    if tasks.len() > MAX_TASKS {
        return Err(format!("task count exceeds {MAX_TASKS}"));
    }
    if binding.len() > MAX_TASK_BYTES {
        return Err("binding exceeds the input size limit".into());
    }

    let mut total_bytes = 0usize;
    let mut indexed = BTreeMap::<String, (&Value, Vec<u8>)>::new();
    for task in tasks {
        let task_id = task
            .get("task_id")
            .and_then(Value::as_str)
            .filter(|id| !id.is_empty() && id.len() <= 1024 && id.is_ascii())
            .ok_or_else(|| {
                "every task requires a nonempty ASCII task_id of at most 1024 bytes".to_string()
            })?;
        let bytes = canonical_bytes(task)?;
        if bytes.len() > MAX_TASK_BYTES {
            return Err(format!("task `{task_id}` exceeds the task size limit"));
        }
        total_bytes = total_bytes
            .checked_add(bytes.len())
            .ok_or_else(|| "total task bytes overflow".to_string())?;
        if total_bytes > MAX_TOTAL_TASK_BYTES {
            return Err("total task input exceeds the size limit".into());
        }
        if indexed.insert(task_id.to_string(), (task, bytes)).is_some() {
            return Err(format!("duplicate task_id `{task_id}`"));
        }
    }

    let output = prepare_directory(output)?;
    let binding_hash = digest(binding.as_bytes());
    let mut completed = BTreeMap::<String, Value>::new();
    let mut missing = Vec::<(String, &Value, String)>::new();
    let mut expected_names = BTreeSet::new();

    // Validate the entire expected inventory before starting any new work.
    for (task_id, (task, task_bytes)) in &indexed {
        let task_hash = digest(task_bytes);
        let filename = format!("task-{}.json", digest(task_id.as_bytes()));
        expected_names.insert(filename.clone());
        let path = output.join(&filename);
        match read_shard(&path, task_id, &task_hash, &binding_hash)? {
            Some(result) => {
                completed.insert(task_id.clone(), result);
            }
            None => missing.push((task_id.clone(), *task, task_hash)),
        }
    }
    validate_directory_inventory(&output, &expected_names)?;

    let limit = stop_after.unwrap_or(usize::MAX);
    let selected = &missing[..missing.len().min(limit)];
    let next = AtomicUsize::new(0);
    let errors = Mutex::new(Vec::<(String, String)>::new());
    let thread_count = workers.min(selected.len());
    thread::scope(|scope| {
        let mut handles = Vec::with_capacity(thread_count);
        for _ in 0..thread_count {
            handles.push(scope.spawn(|| {
                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some((task_id, task, task_hash)) = selected.get(index) else {
                        break;
                    };
                    let result = match catch_unwind(AssertUnwindSafe(|| evaluator(task))) {
                        Ok(Ok(value)) => value,
                        Ok(Err(error)) => {
                            errors
                                .lock()
                                .expect("error mutex poisoned")
                                .push((task_id.clone(), error));
                            continue;
                        }
                        Err(_) => {
                            errors
                                .lock()
                                .expect("error mutex poisoned")
                                .push((task_id.clone(), "evaluator panicked".into()));
                            continue;
                        }
                    };
                    if let Err(error) =
                        publish_shard(&output, task_id, task_hash, &binding_hash, &result)
                    {
                        errors
                            .lock()
                            .expect("error mutex poisoned")
                            .push((task_id.clone(), error));
                    }
                }
            }));
        }
        for handle in handles {
            if handle.join().is_err() {
                errors
                    .lock()
                    .expect("error mutex poisoned")
                    .push((String::new(), "worker panicked".into()));
            }
        }
    });

    let mut errors = errors
        .into_inner()
        .map_err(|_| "worker error mutex poisoned".to_string())?;
    if !errors.is_empty() {
        errors.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
        return Err(errors
            .into_iter()
            .map(|(id, message)| {
                if id.is_empty() {
                    message
                } else {
                    format!("task `{id}`: {message}")
                }
            })
            .collect::<Vec<_>>()
            .join("; "));
    }

    // Re-read published shards through the same validation path. This both
    // returns only durable values and detects an external conflict after work.
    for (task_id, _, task_hash) in selected {
        if completed.contains_key(task_id) {
            continue;
        }
        let filename = format!("task-{}.json", digest(task_id.as_bytes()));
        let result = read_shard(&output.join(filename), task_id, task_hash, &binding_hash)?
            .ok_or_else(|| format!("task `{task_id}` was not durably published"))?;
        completed.insert(task_id.clone(), result);
    }

    let results = completed
        .into_iter()
        .map(|(task_id, result)| TaskResult { task_id, result })
        .collect::<Vec<_>>();
    Ok(Execution {
        complete: results.len() == tasks.len(),
        results,
    })
}

fn canonical_bytes(value: &Value) -> Result<Vec<u8>, String> {
    fn normalize(value: &Value) -> Value {
        match value {
            Value::Object(object) => {
                let sorted = object
                    .iter()
                    .map(|(key, value)| (key.clone(), normalize(value)))
                    .collect::<BTreeMap<_, _>>();
                let mut map = Map::new();
                for (key, value) in sorted {
                    map.insert(key, value);
                }
                Value::Object(map)
            }
            Value::Array(values) => Value::Array(values.iter().map(normalize).collect()),
            _ => value.clone(),
        }
    }
    serde_json::to_vec(&normalize(value))
        .map_err(|error| format!("JSON serialization failed: {error}"))
}

fn digest(bytes: &[u8]) -> String {
    let hash = Sha256::digest(bytes);
    hash.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn prepare_directory(path: &Path) -> Result<PathBuf, String> {
    if path.as_os_str().is_empty() {
        return Err("output directory path is empty".into());
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| format!("cannot resolve current directory: {error}"))?
            .join(path)
    };
    let mut current = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::Prefix(prefix) => current.push(prefix.as_os_str()),
            Component::RootDir => current.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => return Err("output path cannot contain `..`".into()),
            Component::Normal(part) => {
                current.push(part);
                match fs::symlink_metadata(&current) {
                    Ok(metadata) if metadata.file_type().is_symlink() => {
                        return Err(format!(
                            "output path contains symlink component `{}`",
                            current.display()
                        ));
                    }
                    Ok(metadata) if !metadata.is_dir() => {
                        return Err(format!(
                            "output path component is not a directory: `{}`",
                            current.display()
                        ));
                    }
                    Ok(_) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        match fs::create_dir(&current) {
                            Ok(()) => {}
                            Err(create_error)
                                if create_error.kind() == std::io::ErrorKind::AlreadyExists =>
                            {
                                let metadata = fs::symlink_metadata(&current).map_err(|inspect_error| {
                                    format!("cannot revalidate raced output directory `{}`: {inspect_error}", current.display())
                                })?;
                                if metadata.file_type().is_symlink() || !metadata.is_dir() {
                                    return Err(format!(
                                        "raced output component is not a real directory: `{}`",
                                        current.display()
                                    ));
                                }
                            }
                            Err(create_error) => {
                                return Err(format!(
                                    "cannot create output directory `{}`: {create_error}",
                                    current.display()
                                ));
                            }
                        }
                    }
                    Err(error) => {
                        return Err(format!(
                            "cannot inspect output path `{}`: {error}",
                            current.display()
                        ));
                    }
                }
            }
        }
    }
    Ok(current)
}

fn read_shard(
    path: &Path,
    task_id: &str,
    task_hash: &str,
    binding_hash: &str,
) -> Result<Option<Value>, String> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!(
                "cannot inspect shard `{}`: {error}",
                path.display()
            ));
        }
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(format!("shard is not a regular file: `{}`", path.display()));
    }
    if metadata.len() as usize > MAX_SHARD_BYTES {
        return Err(format!("shard exceeds size limit: `{}`", path.display()));
    }
    let mut bytes = Vec::with_capacity(metadata.len().min(MAX_SHARD_BYTES as u64) as usize);
    File::open(path)
        .and_then(|file| {
            file.take(MAX_SHARD_BYTES as u64 + 1)
                .read_to_end(&mut bytes)
        })
        .map_err(|error| format!("cannot read shard `{}`: {error}", path.display()))?;
    if bytes.len() > MAX_SHARD_BYTES {
        return Err(format!("shard exceeds size limit: `{}`", path.display()));
    }
    let envelope: Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("corrupt shard `{}`: {error}", path.display()))?;
    let expected = [
        ("schema_version", SHARD_SCHEMA),
        ("task_id", task_id),
        ("task_hash", task_hash),
        ("binding_hash", binding_hash),
    ];
    for (field, value) in expected {
        if envelope.get(field).and_then(Value::as_str) != Some(value) {
            return Err(format!(
                "shard `{}` has incompatible `{field}`",
                path.display()
            ));
        }
    }
    let result = envelope
        .get("result")
        .ok_or_else(|| format!("shard `{}` has no result", path.display()))?;
    let result_bytes = canonical_bytes(result)?;
    if envelope.get("result_hash").and_then(Value::as_str) != Some(digest(&result_bytes).as_str()) {
        return Err(format!(
            "shard `{}` has invalid result hash",
            path.display()
        ));
    }
    let allowed = [
        "schema_version",
        "task_id",
        "task_hash",
        "binding_hash",
        "result",
        "result_hash",
    ];
    if envelope.as_object().is_none_or(|object| {
        object.len() != allowed.len() || object.keys().any(|key| !allowed.contains(&key.as_str()))
    }) {
        return Err(format!(
            "shard `{}` has unknown or missing fields",
            path.display()
        ));
    }
    Ok(Some(result.clone()))
}

fn publish_shard(
    output: &Path,
    task_id: &str,
    task_hash: &str,
    binding_hash: &str,
    result: &Value,
) -> Result<(), String> {
    let filename = format!("task-{}.json", digest(task_id.as_bytes()));
    let final_path = output.join(filename);
    let result_bytes = canonical_bytes(result)?;
    if result_bytes.len() > MAX_SHARD_BYTES / 2 {
        return Err("result exceeds shard size limit".into());
    }
    let mut envelope = Map::new();
    envelope.insert("schema_version".into(), Value::String(SHARD_SCHEMA.into()));
    envelope.insert("task_id".into(), Value::String(task_id.into()));
    envelope.insert("task_hash".into(), Value::String(task_hash.into()));
    envelope.insert("binding_hash".into(), Value::String(binding_hash.into()));
    envelope.insert("result".into(), result.clone());
    envelope.insert("result_hash".into(), Value::String(digest(&result_bytes)));
    let bytes = canonical_bytes(&Value::Object(envelope))?;
    if bytes.len() > MAX_SHARD_BYTES {
        return Err("serialized shard exceeds size limit".into());
    }
    let temp_path = output.join(format!(
        "{TEMP_PREFIX}{}-{}-{}",
        std::process::id(),
        TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed),
        digest(task_id.as_bytes())
    ));
    let mut temp = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp_path)
        .map_err(|error| {
            format!(
                "cannot create temporary shard `{}`: {error}",
                temp_path.display()
            )
        })?;
    let write_result = temp.write_all(&bytes).and_then(|()| temp.sync_all());
    drop(temp);
    if let Err(error) = write_result {
        let _ = fs::remove_file(&temp_path);
        return Err(format!("cannot durably write temporary shard: {error}"));
    }
    match fs::hard_link(&temp_path, &final_path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let existing = read_shard(&final_path, task_id, task_hash, binding_hash)?
                .ok_or_else(|| "published shard disappeared during concurrent write".to_string())?;
            if canonical_bytes(&existing)? != result_bytes {
                let _ = fs::remove_file(&temp_path);
                return Err(format!(
                    "conflicting result already published for task `{task_id}`"
                ));
            }
        }
        Err(error) => {
            let _ = fs::remove_file(&temp_path);
            return Err(format!("cannot publish shard without replacement: {error}"));
        }
    }
    fs::remove_file(&temp_path)
        .map_err(|error| format!("cannot remove temporary shard: {error}"))?;
    File::open(output)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("cannot sync shard directory: {error}"))?;
    Ok(())
}

fn validate_directory_inventory(output: &Path, expected: &BTreeSet<String>) -> Result<(), String> {
    for entry in
        fs::read_dir(output).map_err(|error| format!("cannot read shard directory: {error}"))?
    {
        let entry = entry.map_err(|error| format!("cannot read shard directory entry: {error}"))?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with(TEMP_PREFIX) {
            let file_type = entry
                .file_type()
                .map_err(|error| format!("cannot inspect temporary `{name}`: {error}"))?;
            if file_type.is_symlink() || !file_type.is_file() {
                return Err(format!("temporary entry is not a regular file: `{name}`"));
            }
            if entry
                .metadata()
                .map_err(|error| format!("cannot inspect temporary `{name}`: {error}"))?
                .len()
                > MAX_SHARD_BYTES as u64
            {
                return Err(format!("temporary entry exceeds size limit: `{name}`"));
            }
            continue;
        }
        if !expected.contains(name.as_ref()) {
            return Err(format!("unexpected file in shard directory: `{name}`"));
        }
        if entry
            .file_type()
            .map_err(|error| format!("cannot inspect `{name}`: {error}"))?
            .is_symlink()
        {
            return Err(format!("symlink shard is forbidden: `{name}`"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join(".artifacts/c5-workers/test-output");
        fs::create_dir_all(&root).unwrap();
        let path = root.join(format!("c5-workers-{label}-{}-{nonce}", std::process::id()));
        fs::create_dir(&path).unwrap();
        path
    }

    fn tasks(count: usize) -> Vec<Value> {
        (0..count)
            .map(|index| serde_json::json!({"task_id": format!("t{index:03}"), "x": index}))
            .collect()
    }

    #[test]
    fn worker_count_and_completion_order_do_not_change_shards_or_results() {
        let one = temp_dir("one");
        let two = temp_dir("two");
        let many = temp_dir("many");
        let input = tasks(12);
        let first = execute(&input, 1, &one, "binding", None, |task| {
            Ok(serde_json::json!({"value": task["x"].as_u64().unwrap() * 3}))
        })
        .unwrap();
        let middle = execute(&input, 2, &two, "binding", None, |task| {
            let id = task["x"].as_u64().unwrap();
            std::thread::sleep(std::time::Duration::from_millis((id % 3) * 2));
            Ok(serde_json::json!({"value": id * 3}))
        })
        .unwrap();
        let second = execute(&input, 4, &many, "binding", None, |task| {
            let id = task["x"].as_u64().unwrap();
            std::thread::sleep(std::time::Duration::from_millis((11 - id) * 2));
            Ok(serde_json::json!({"value": id * 3}))
        })
        .unwrap();
        assert_eq!(first, second);
        assert_eq!(first, middle);
        for task in &input {
            let name = format!(
                "task-{}.json",
                digest(task["task_id"].as_str().unwrap().as_bytes())
            );
            assert_eq!(
                fs::read(one.join(&name)).unwrap(),
                fs::read(many.join(name)).unwrap()
            );
            let name = format!(
                "task-{}.json",
                digest(task["task_id"].as_str().unwrap().as_bytes())
            );
            assert_eq!(
                fs::read(one.join(&name)).unwrap(),
                fs::read(two.join(name)).unwrap()
            );
        }
        let _ = fs::remove_dir_all(one);
        let _ = fs::remove_dir_all(two);
        let _ = fs::remove_dir_all(many);
    }

    #[test]
    fn stop_and_resume_matches_complete_execution_and_reuses_shards() {
        let resumed = temp_dir("resume");
        let baseline = temp_dir("baseline");
        let input = tasks(8);
        let evaluator = |task: &Value| Ok(serde_json::json!({"x": task["x"]}));
        let partial = execute(&input, 3, &resumed, "same", Some(3), evaluator).unwrap();
        assert!(!partial.complete);
        assert_eq!(partial.results.len(), 3);
        let calls = AtomicUsize::new(0);
        let complete = execute(&input, 4, &resumed, "same", None, |task| {
            calls.fetch_add(1, Ordering::SeqCst);
            evaluator(task)
        })
        .unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 5);
        assert!(complete.complete);
        let full = execute(&input, 2, &baseline, "same", None, evaluator).unwrap();
        assert_eq!(complete, full);
        for task in &input {
            let name = format!(
                "task-{}.json",
                digest(task["task_id"].as_str().unwrap().as_bytes())
            );
            assert_eq!(
                fs::read(resumed.join(&name)).unwrap(),
                fs::read(baseline.join(name)).unwrap()
            );
        }
        let _ = fs::remove_dir_all(resumed);
        let _ = fs::remove_dir_all(baseline);
    }

    #[test]
    fn rejects_duplicate_ids_corrupt_shards_and_binding_drift() {
        let output = temp_dir("reject");
        let input = tasks(2);
        let duplicate = vec![input[0].clone(), input[0].clone()];
        assert!(execute(&duplicate, 1, &output, "b", None, |_| Ok(Value::Null)).is_err());
        execute(&input, 1, &output, "b", None, |_| Ok(Value::Null)).unwrap();
        let name = format!(
            "task-{}.json",
            digest(input[0]["task_id"].as_str().unwrap().as_bytes())
        );
        let shard_path = output.join(name);
        let valid_bytes = fs::read(&shard_path).unwrap();
        let calls = AtomicUsize::new(0);
        assert!(
            execute(&input, 1, &output, "changed", None, |_| {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok(Value::Null)
            })
            .is_err()
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);

        for (field, value) in [
            ("schema_version", Value::String("unknown.v9".into())),
            ("task_hash", Value::String("00".repeat(32))),
            ("result_hash", Value::String("00".repeat(32))),
        ] {
            let mut shard: Value = serde_json::from_slice(&valid_bytes).unwrap();
            shard[field] = value;
            fs::write(&shard_path, canonical_bytes(&shard).unwrap()).unwrap();
            let calls = AtomicUsize::new(0);
            assert!(
                execute(&input, 1, &output, "b", None, |_| {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Ok(Value::Null)
                })
                .is_err()
            );
            assert_eq!(calls.load(Ordering::SeqCst), 0, "{field}");
        }
        let mut unknown: Value = serde_json::from_slice(&valid_bytes).unwrap();
        unknown["unexpected"] = Value::Bool(true);
        fs::write(&shard_path, canonical_bytes(&unknown).unwrap()).unwrap();
        assert!(execute(&input, 1, &output, "b", None, |_| Ok(Value::Null)).is_err());

        fs::write(&shard_path, &valid_bytes).unwrap();
        let mut drifted = input.clone();
        drifted[0]["x"] = Value::from(999);
        let calls = AtomicUsize::new(0);
        assert!(
            execute(&drifted, 1, &output, "b", None, |_| {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok(Value::Null)
            })
            .is_err()
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);

        fs::write(output.join("unrecognized.json"), b"{}").unwrap();
        let calls = AtomicUsize::new(0);
        assert!(
            execute(&input, 1, &output, "b", None, |_| {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok(Value::Null)
            })
            .is_err()
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        let _ = fs::remove_dir_all(output);
    }

    #[test]
    fn callback_failure_does_not_publish_failed_task() {
        let output = temp_dir("failure");
        let input = tasks(1);
        assert!(execute(&input, 1, &output, "b", None, |_| Err("expected".into())).is_err());
        assert_eq!(fs::read_dir(&output).unwrap().count(), 0);
        let _ = fs::remove_dir_all(output);
    }

    #[test]
    fn callback_panic_is_caught_and_remaining_selected_tasks_finish() {
        let output = temp_dir("panic");
        let input = tasks(4);
        let calls = AtomicUsize::new(0);
        assert!(
            execute(&input, 1, &output, "b", None, |task| {
                calls.fetch_add(1, Ordering::SeqCst);
                if task["task_id"] == "t000" {
                    panic!("injected evaluator panic");
                }
                Ok(serde_json::json!({"x": task["x"]}))
            })
            .is_err()
        );
        assert_eq!(calls.load(Ordering::SeqCst), 4);
        assert_eq!(fs::read_dir(&output).unwrap().count(), 3);
        let _ = fs::remove_dir_all(output);
    }

    #[test]
    fn accepts_regular_orphan_temp_but_rejects_temp_directories_and_oversized_shards() {
        let output = temp_dir("temps");
        let input = tasks(1);
        execute(&input, 1, &output, "b", None, |_| Ok(Value::Null)).unwrap();
        fs::write(output.join(".c5tmp-orphan"), b"partial").unwrap();
        let calls = AtomicUsize::new(0);
        assert!(
            execute(&input, 1, &output, "b", None, |_| {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok(Value::Null)
            })
            .unwrap()
            .complete
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        fs::remove_file(output.join(".c5tmp-orphan")).unwrap();
        fs::create_dir(output.join(".c5tmp-directory")).unwrap();
        assert!(execute(&input, 1, &output, "b", None, |_| Ok(Value::Null)).is_err());
        fs::remove_dir(output.join(".c5tmp-directory")).unwrap();
        let name = format!(
            "task-{}.json",
            digest(input[0]["task_id"].as_str().unwrap().as_bytes())
        );
        fs::write(output.join(name), vec![b'x'; MAX_SHARD_BYTES + 1]).unwrap();
        assert!(execute(&input, 1, &output, "b", None, |_| Ok(Value::Null)).is_err());
        let _ = fs::remove_dir_all(output);
    }

    #[test]
    fn concurrent_equivalent_writers_converge_on_one_shard() {
        let output = temp_dir("concurrent");
        let input = tasks(1);
        thread::scope(|scope| {
            let left = scope.spawn(|| {
                execute(&input, 1, &output, "same", None, |task| {
                    Ok(serde_json::json!({"x": task["x"]}))
                })
            });
            let right = scope.spawn(|| {
                execute(&input, 1, &output, "same", None, |task| {
                    Ok(serde_json::json!({"x": task["x"]}))
                })
            });
            assert_eq!(
                left.join().unwrap().unwrap(),
                right.join().unwrap().unwrap()
            );
        });
        assert_eq!(fs::read_dir(&output).unwrap().count(), 1);
        let _ = fs::remove_dir_all(output);
    }

    #[test]
    fn concurrent_creation_of_output_directory_is_revalidated() {
        let parent = temp_dir("directory-race");
        let output = parent.join("new-output");
        let input = tasks(1);
        thread::scope(|scope| {
            let left =
                scope.spawn(|| execute(&input, 1, &output, "same", None, |_| Ok(Value::Null)));
            let right =
                scope.spawn(|| execute(&input, 1, &output, "same", None, |_| Ok(Value::Null)));
            assert_eq!(
                left.join().unwrap().unwrap(),
                right.join().unwrap().unwrap()
            );
        });
        assert_eq!(fs::read_dir(&output).unwrap().count(), 1);
        let _ = fs::remove_dir_all(parent);
    }

    #[test]
    fn enforces_worker_and_task_limits_and_rejects_symlink_output() {
        let parent = temp_dir("limits");
        assert!(execute(&[], 0, &parent.join("zero"), "b", None, |_| Ok(Value::Null)).is_err());
        #[cfg(unix)]
        {
            let target = parent.join("target");
            fs::create_dir(&target).unwrap();
            let link = parent.join("link");
            std::os::unix::fs::symlink(&target, &link).unwrap();
            assert!(execute(&[], 1, &link, "b", None, |_| Ok(Value::Null)).is_err());
        }
        let _ = fs::remove_dir_all(parent);
    }
}
