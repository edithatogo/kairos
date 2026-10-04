//! Private bounded external sort for C0 trace records.
use super::ingestion_normalize::order_key;
use serde_json::Value;
use std::{
    cmp::Ordering,
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering as AtomicOrdering},
};

const SERIALIZATION_VERSION: &str = "serde-json-btree-ndjson-v1";
static WORKSPACE_NONCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug)]
pub(crate) struct SortLimits {
    pub(crate) max_run_rows: usize,
    pub(crate) max_run_bytes: usize,
    pub(crate) max_record_bytes: usize,
    pub(crate) merge_fan_in: usize,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct SortReport {
    pub(crate) rows: u64,
    pub(crate) runs: usize,
    pub(crate) merge_passes: usize,
}
#[derive(Debug)]
struct Run {
    path: PathBuf,
    rows: u64,
}
struct Workspace(PathBuf);
impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub(crate) fn external_sort<I, F>(
    records: I,
    scratch: &Path,
    limits: SortLimits,
    mut sink: F,
) -> Result<SortReport, String>
where
    I: Iterator<Item = Result<Value, String>>,
    F: FnMut(Value) -> Result<(), String>,
{
    validate_limits(limits)?;
    let scratch =
        fs::canonicalize(scratch).map_err(|e| format!("invalid scratch directory: {e}"))?;
    if !fs::metadata(&scratch).map_err(|e| e.to_string())?.is_dir() {
        return Err("scratch path must be a directory".into());
    }
    let workspace = Workspace(create_workspace(&scratch)?);
    let mut report = SortReport::default();
    let mut runs = Vec::new();
    let mut batch: Vec<(super::trace_order::TraceOrderKeyV1, Value)> = Vec::new();
    let mut batch_bytes = 0usize;
    for record in records {
        let value = record?;
        let key = order_key(&value)?;
        let encoded =
            serde_json::to_vec(&value).map_err(|e| format!("record serialization failed: {e}"))?;
        if encoded.len() > limits.max_record_bytes {
            return Err("record exceeds max_record_bytes".into());
        }
        let line_bytes = encoded
            .len()
            .checked_add(1)
            .ok_or("record byte count overflow")?;
        if line_bytes > limits.max_run_bytes {
            return Err("record cannot fit within max_run_bytes".into());
        }
        let would_bytes = batch_bytes
            .checked_add(line_bytes)
            .ok_or("run byte count overflow")?;
        if !batch.is_empty()
            && (batch.len() >= limits.max_run_rows || would_bytes > limits.max_run_bytes)
        {
            check_run_metadata(&runs, &workspace.0, limits.max_run_bytes)?;
            runs.push(write_initial_run(&workspace.0, &mut batch, runs.len())?);
            batch_bytes = 0;
        }
        batch_bytes = batch_bytes
            .checked_add(line_bytes)
            .ok_or("run byte count overflow")?;
        batch.push((key, value));
        report.rows = report.rows.checked_add(1).ok_or("row count overflow")?;
    }
    if !batch.is_empty() {
        check_run_metadata(&runs, &workspace.0, limits.max_run_bytes)?;
        runs.push(write_initial_run(&workspace.0, &mut batch, runs.len())?);
    }
    report.runs = runs.len();
    if runs.is_empty() {
        return Ok(report);
    }
    let mut level = 0usize;
    while runs.len() > limits.merge_fan_in {
        let mut next = Vec::new();
        let mut made_merge = false;
        for chunk in runs.chunks(limits.merge_fan_in) {
            if chunk.len() == 1 {
                next.push(Run {
                    path: chunk[0].path.clone(),
                    rows: chunk[0].rows,
                });
                continue;
            }
            let path = workspace
                .0
                .join(format!("merge-{level}-{}.ndjson", next.len()));
            let rows = merge_to_path(chunk, &path, limits.max_record_bytes)?;
            next.push(Run { path, rows });
            made_merge = true;
            for run in chunk {
                fs::remove_file(&run.path).map_err(|e| format!("remove merged run failed: {e}"))?;
            }
        }
        if made_merge {
            report.merge_passes = report
                .merge_passes
                .checked_add(1)
                .ok_or("merge pass count overflow")?;
        }
        runs = next;
        level = level.checked_add(1).ok_or("merge level overflow")?;
    }
    if runs.len() == 1 {
        emit_run(&runs[0], limits.max_record_bytes, &mut sink)?;
    } else {
        merge_to_sink(&runs, limits.max_record_bytes, &mut sink)?;
        report.merge_passes = report
            .merge_passes
            .checked_add(1)
            .ok_or("merge pass count overflow")?;
    }
    let _ = SERIALIZATION_VERSION; // version is part of the private serialization contract.
    drop(workspace);
    Ok(report)
}
fn check_run_metadata(runs: &[Run], workspace: &Path, cap: usize) -> Result<(), String> {
    let per = std::mem::size_of::<Run>()
        .checked_add(workspace.as_os_str().len())
        .and_then(|v| v.checked_add(64))
        .ok_or("run metadata size overflow")?;
    if runs
        .len()
        .checked_add(1)
        .and_then(|n| n.checked_mul(per))
        .ok_or("run metadata size overflow")?
        > cap
    {
        return Err("run metadata exceeds max_run_bytes".into());
    }
    Ok(())
}
fn validate_limits(l: SortLimits) -> Result<(), String> {
    if l.max_run_rows == 0 || l.max_run_bytes == 0 || l.max_record_bytes == 0 {
        return Err("sort limits must be nonzero".into());
    }
    if l.merge_fan_in < 2 {
        return Err("merge_fan_in must be at least two".into());
    }
    l.max_record_bytes
        .checked_add(2)
        .ok_or("max_record_bytes overflow")?;
    Ok(())
}
fn create_workspace(scratch: &Path) -> Result<PathBuf, String> {
    for _ in 0..128 {
        let nonce = WORKSPACE_NONCE.fetch_add(1, AtomicOrdering::Relaxed);
        let path = scratch.join(format!(".kairos-sort-{}-{nonce}", std::process::id()));
        match fs::create_dir(&path) {
            Ok(()) => {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Err(e) = fs::set_permissions(&path, fs::Permissions::from_mode(0o700)) {
                        let _ = fs::remove_dir(&path);
                        return Err(e.to_string());
                    }
                }
                return Ok(path);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(format!("create sort workspace failed: {e}")),
        }
    }
    Err("unable to allocate unique sort workspace".into())
}
fn write_initial_run(
    dir: &Path,
    batch: &mut Vec<(super::trace_order::TraceOrderKeyV1, Value)>,
    index: usize,
) -> Result<Run, String> {
    let rows = u64::try_from(batch.len()).map_err(|_| "run row count overflow")?;
    batch.sort_by(|a, b| a.0.cmp(&b.0));
    let path = dir.join(format!("run-0-{index}.ndjson"));
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|e| format!("create run failed: {e}"))?;
    let mut w = BufWriter::new(file);
    for (_, value) in batch.drain(..) {
        write_record(&mut w, &value)?;
    }
    w.flush().map_err(|e| format!("flush run failed: {e}"))?;
    Ok(Run { path, rows })
}
fn write_record<W: Write>(w: &mut W, v: &Value) -> Result<(), String> {
    let bytes = serde_json::to_vec(v).map_err(|e| e.to_string())?;
    w.write_all(&bytes)
        .and_then(|_| w.write_all(b"\n"))
        .map_err(|e| format!("write run failed: {e}"))
}
fn merge_to_path(runs: &[Run], path: &Path, max_record: usize) -> Result<u64, String> {
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| format!("create merge run failed: {e}"))?;
    let mut writer = BufWriter::new(file);
    let rows = merge(runs, max_record, |v| write_record(&mut writer, &v))?;
    writer
        .flush()
        .map_err(|e| format!("flush merge run failed: {e}"))?;
    Ok(rows)
}
fn merge_to_sink<F: FnMut(Value) -> Result<(), String>>(
    runs: &[Run],
    max_record: usize,
    sink: &mut F,
) -> Result<u64, String> {
    merge(runs, max_record, sink)
}
fn emit_run<F: FnMut(Value) -> Result<(), String>>(
    run: &Run,
    max_record: usize,
    sink: &mut F,
) -> Result<u64, String> {
    let f = File::open(&run.path).map_err(|e| format!("open run failed: {e}"))?;
    let mut reader = BufReader::new(f);
    let mut rows = 0u64;
    while let Some((value, _)) = read_record(&mut reader, max_record)? {
        sink(value)?;
        rows = rows.checked_add(1).ok_or("row count overflow")?;
    }
    if rows != run.rows {
        return Err("run row count mismatch".into());
    }
    Ok(rows)
}
fn merge<F: FnMut(Value) -> Result<(), String>>(
    runs: &[Run],
    max_record: usize,
    mut consume: F,
) -> Result<u64, String> {
    if runs.is_empty() {
        return Ok(0);
    }
    let mut readers = Vec::with_capacity(runs.len());
    for run in runs {
        readers.push(BufReader::new(
            File::open(&run.path).map_err(|e| format!("open run failed: {e}"))?,
        ));
    }
    let mut heads: Vec<Option<(Value, super::trace_order::TraceOrderKeyV1)>> =
        Vec::with_capacity(runs.len());
    for reader in &mut readers {
        heads.push(read_record(reader, max_record)?);
    }
    let mut emitted = 0u64;
    loop {
        let mut best: Option<usize> = None;
        for i in 0..heads.len() {
            if let Some((_, key)) = &heads[i] {
                let choose = best.is_none_or(|j| match &heads[j] {
                    Some((_, other)) => key.cmp(other) == Ordering::Less,
                    None => true,
                });
                if choose {
                    best = Some(i);
                }
            }
        }
        let Some(i) = best else { break };
        let (value, _) = heads[i].take().expect("selected head exists");
        consume(value)?;
        emitted = emitted.checked_add(1).ok_or("merge row count overflow")?;
        heads[i] = read_record(&mut readers[i], max_record)?;
    }
    let expected = runs.iter().try_fold(0u64, |sum, r| {
        sum.checked_add(r.rows).ok_or("merge row count overflow")
    })?;
    if emitted != expected {
        return Err("merge row count mismatch".into());
    }
    Ok(emitted)
}
fn read_record<R: Read>(
    reader: &mut BufReader<R>,
    max_record: usize,
) -> Result<Option<(Value, super::trace_order::TraceOrderKeyV1)>, String> {
    let cap = max_record
        .checked_add(2)
        .ok_or("record read limit overflow")?;
    let mut bytes = Vec::new();
    let got = reader
        .take(u64::try_from(cap).map_err(|_| "record read limit conversion overflow")?)
        .read_until(b'\n', &mut bytes)
        .map_err(|e| format!("read run failed: {e}"))?;
    if got == 0 {
        return Ok(None);
    }
    if bytes.len() > max_record.saturating_add(1) {
        return Err("run record exceeds max_record_bytes".into());
    }
    if bytes.last() != Some(&b'\n') {
        return Err("truncated/corrupt run record".into());
    }
    bytes.pop();
    if bytes.is_empty() {
        return Err("empty/corrupt run record".into());
    }
    let value: Value =
        serde_json::from_slice(&bytes).map_err(|e| format!("corrupt run JSON: {e}"))?;
    if serde_json::to_vec(&value).map_err(|e| e.to_string())? != bytes {
        return Err("noncanonical run JSON".into());
    }
    let key = order_key(&value)?;
    Ok(Some((value, key)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingestion_normalize::{Limits, Normalizer};
    use serde_json::json;
    use std::{
        fs,
        io::Write,
        time::{SystemTime, UNIX_EPOCH},
    };
    fn temp() -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "kairos-sort-test-{}-{}",
            std::process::id(),
            WORKSPACE_NONCE.fetch_add(1, AtomicOrdering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        p
    }
    fn limits(rows: usize, fanin: usize) -> SortLimits {
        SortLimits {
            max_run_rows: rows,
            max_run_bytes: 1 << 20,
            max_record_bytes: 1 << 16,
            merge_fan_in: fanin,
        }
    }
    fn generated() -> Vec<Value> {
        let template = json!({"profile_version":"c11.synthetic-map-v1","dataset_id":"d","mapping_version":"m","origin_utc":"2020-01-01T00:00:00Z","case_key_field":"case","source_family":"fixture","shape":"wide","event_bindings":[{"source_event_type":"ARRIVAL","kind":"arrival","rank":"2","occurrence_index":0,"occurrence_field":"at","key_field":"id","order_field":"seq"}],"rows":[]});
        let mut n = Normalizer::new(
            template,
            Limits {
                max_chunk_rows: 10,
                max_chunk_bytes: 8192,
                max_identities: 10,
            },
        )
        .unwrap();
        let mut out = Vec::new();
        for (id, time, seq) in [
            ("z", "2020-01-01T00:00:03Z", 3),
            ("a", "2020-01-01T00:00:01Z", 1),
            ("b", "2020-01-01T00:00:02Z", 2),
        ] {
            let row = json!({"case":"c","id":id,"seq":seq,"at":{"raw":time,"representation":"RFC3339","precision":"second","lineage":{"status":"observed"}}});
            out.extend(n.push(vec![row]).unwrap().events);
        }
        out
    }
    fn run(values: Vec<Value>, dir: &Path, l: SortLimits) -> (SortReport, Vec<Value>) {
        let mut out = Vec::new();
        let r = external_sort(values.into_iter().map(Ok), dir, l, |v| {
            out.push(v);
            Ok(())
        })
        .unwrap();
        (r, out)
    }
    #[test]
    fn actual_normalized_values_are_hash_identical_across_orders_and_run_layouts() {
        let base = generated();
        let scratch = temp();
        let (_, a) = run(base.clone(), &scratch, limits(1, 2));
        let mut shuffled = base.clone();
        shuffled.reverse();
        let (_, b) = run(shuffled, &scratch, limits(2, 2));
        let (_, c) = run(base, &scratch, limits(100, 3));
        assert_eq!(
            serde_json::to_vec(&a).unwrap(),
            serde_json::to_vec(&b).unwrap()
        );
        assert_eq!(
            serde_json::to_vec(&a).unwrap(),
            serde_json::to_vec(&c).unwrap()
        );
        assert_eq!(
            a.iter()
                .map(|v| v["source_event_key"].as_str().unwrap())
                .collect::<Vec<_>>(),
            vec!["a", "b", "z"]
        );
        fs::remove_dir_all(scratch).unwrap();
    }
    #[test]
    fn bounded_fanin_requires_multiple_merge_passes_and_handles_large_rank() {
        let mut values = generated();
        for (i, v) in values.iter_mut().enumerate() {
            v["event_kind_rank"] = json!("1".to_owned() + &"0".repeat(80 - i));
        }
        let scratch = temp();
        let (report, out) = run(values, &scratch, limits(1, 2));
        assert_eq!(report.runs, 3);
        assert!(report.merge_passes >= 2);
        assert_eq!(out.len(), 3);
        fs::remove_dir_all(scratch).unwrap();
    }
    #[test]
    fn iterator_sink_oversize_invalid_key_and_temp_corruption_fail_with_cleanup() {
        let scratch = temp();
        let sentinel = scratch.join("keep");
        fs::write(&sentinel, b"untouched").unwrap();
        let err = external_sort(
            vec![Ok(generated()[0].clone()), Err("source failure".into())].into_iter(),
            &scratch,
            limits(1, 2),
            |_| Ok(()),
        )
        .unwrap_err();
        assert!(err.contains("source failure"));
        let mut invalid = generated()[0].clone();
        invalid["relative_ticks"] = json!("-1");
        assert!(external_sort(
            vec![Ok(invalid)].into_iter(),
            &scratch,
            limits(1, 2),
            |_| Ok(())
        )
        .is_err());
        let mut l = limits(1, 2);
        l.max_record_bytes = 32;
        assert!(external_sort(
            vec![Ok(generated()[0].clone())].into_iter(),
            &scratch,
            l,
            |_| Ok(())
        )
        .is_err());
        assert_eq!(fs::read(&sentinel).unwrap(), b"untouched");
        assert_eq!(fs::read_dir(&scratch).unwrap().count(), 1);
        let corrupt = scratch.join("corrupt");
        let mut f = File::create(&corrupt).unwrap();
        f.write_all(b"{broken}\n").unwrap();
        let mut reader = BufReader::new(File::open(&corrupt).unwrap());
        assert!(read_record(&mut reader, 100).is_err());
        fs::remove_dir_all(scratch).unwrap();
    }
    #[test]
    fn sink_failure_and_bad_limits_cleanup_workspace() {
        let scratch = temp();
        fs::write(scratch.join("sentinel"), b"x").unwrap();
        assert!(external_sort(
            generated().into_iter().map(Ok),
            &scratch,
            limits(1, 2),
            |_| Err("sink rejected".into())
        )
        .is_err());
        assert_eq!(fs::read_dir(&scratch).unwrap().count(), 1);
        assert!(external_sort(
            std::iter::empty(),
            &scratch,
            SortLimits {
                merge_fan_in: 1,
                ..limits(1, 2)
            },
            |_| Ok(())
        )
        .is_err());
        let _ = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        fs::remove_dir_all(scratch).unwrap();
    }
}
