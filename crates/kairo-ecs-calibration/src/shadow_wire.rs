//! Bounded private C3 run inventory transport; no artifact-selected model code.
use crate::seed_map::checkpoint_wire::{encode_key, SeedWireLimits};
use crate::shadow::{
    LedgerSnapshot, LimitReason, ProbeCheckpoint, ProbeOutcome, ProbeSpec, RunnerCheckpoint,
    SavedProbeState, ShadowError, Transition,
};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

const MAGIC: &[u8; 8] = b"KC3RUN01";
const HEADER: usize = 8 + 4 + 32 + 8 + 8;
const ROW_FIXED: usize = 8 + 32 + 8 + 16 + 1 + 8;

#[derive(Clone, Copy)]
pub(crate) struct WireLimits {
    pub max_wire_bytes: usize,
    pub max_probes: usize,
    pub max_id_bytes: usize,
    pub max_image_bytes: usize,
    pub max_reason_bytes: usize,
}

pub(crate) struct RunImage {
    pub ledger_frontier: u64,
    pub runner: RunnerCheckpoint,
}

struct Hash(Sha256);
impl Hash {
    fn bytes(&mut self, value: &[u8]) {
        self.0.update((value.len() as u64).to_le_bytes());
        self.0.update(value);
    }
    fn text(&mut self, value: &str) {
        self.bytes(value.as_bytes());
    }
    fn number(&mut self, value: u128) {
        self.0.update(value.to_le_bytes());
    }
    fn optional_tick(&mut self, value: Option<u128>) {
        self.0.update([u8::from(value.is_some())]);
        if let Some(value) = value {
            self.number(value);
        }
    }
}

/// Includes every immutable field, even when a containing digest is present.
fn fingerprint(spec: &ProbeSpec, snapshot: &LedgerSnapshot) -> Result<[u8; 32], ShadowError> {
    let mut h = Hash(Sha256::new());
    h.text("kairos.c3.probe-binding.v1");
    for text in [
        &spec.id,
        &spec.run_id,
        &spec.candidate_id,
        &spec.anchor_event,
        &spec.input.target,
        &spec.input.fidelity,
        &spec.key.study_id,
        &spec.key.dataset_id,
        &spec.key.scenario_id,
        &spec.key.seed_schedule_id,
        &spec.key.replication_id,
        &spec.key.case_key,
        &spec.key.task_key,
        &spec.key.endpoint,
        &spec.key.seed_purpose,
        &spec.key.seed_map_ref,
        &spec.key.mapping_version,
    ] {
        h.text(text);
    }
    h.number(u128::from(spec.key.occurrence));
    h.0.update([u8::from(spec.target_event.is_some())]);
    if let Some(target) = &spec.target_event {
        h.text(target);
    }
    h.optional_tick(spec.observed_target);
    h.number(spec.budget.horizon);
    h.number(u128::from(spec.budget.max_events));
    h.bytes(&spec.input.parameter_hash);
    h.bytes(&spec.input.adapter_hash);
    let ids = spec
        .input
        .seed_key
        .checkpoint_identifier_bytes()
        .map_err(|_| ShadowError::LimitExceeded)?;
    let key = encode_key(
        &spec.input.seed_key,
        SeedWireLimits {
            max_entries: 1,
            max_identifier_bytes: ids,
            max_wire_bytes: ids.checked_add(1024).ok_or(ShadowError::LimitExceeded)?,
        },
    )
    .map_err(|_| ShadowError::InvalidInput("seed key cannot be encoded"))?;
    h.bytes(&key);
    h.number(snapshot.frontier as u128);
    h.number(snapshot.at);
    h.text(&snapshot.anchor_event);
    h.bytes(&snapshot.digest);
    h.0.update([u8::from(snapshot.resource_feasible)]);
    h.number(snapshot.visible_events.len() as u128);
    for event in &snapshot.visible_events {
        h.number(event.order.relative_ticks);
        h.text(&event.order.case_key);
        h.number(u128::from(event.order.occurrence));
        h.text(event.order.event_kind_rank.as_str());
        h.text(&event.order.source_event_key);
        h.number(u128::from(event.order.source_order));
        h.optional_tick(event.available_at);
        h.0.update([u8::from(event.source_defined)]);
        match &event.transition {
            Transition::None => h.0.update([0]),
            Transition::Acquire {
                resource,
                claim,
                units,
            } => {
                h.0.update([1]);
                h.text(resource);
                h.text(claim);
                h.number(u128::from(*units));
            }
            Transition::Release { resource, claim } => {
                h.0.update([2]);
                h.text(resource);
                h.text(claim);
            }
        }
        h.bytes(&event.payload);
    }
    h.number(snapshot.resources.len() as u128);
    for (id, resource) in &snapshot.resources {
        h.text(id);
        h.number(u128::from(resource.capacity));
        h.number(resource.claims.len() as u128);
        for (claim, units) in &resource.claims {
            h.text(claim);
            h.number(u128::from(*units));
        }
    }
    h.number(snapshot.assumptions.len() as u128);
    for assumption in &snapshot.assumptions {
        h.text(assumption);
    }
    Ok(h.0.finalize().into())
}

fn state_parts(state: &SavedProbeState) -> (u8, usize) {
    match state {
        SavedProbeState::Pending(bytes) => (1, bytes.len()),
        SavedProbeState::Terminal(ProbeOutcome::Completed { .. }) => (2, 16),
        SavedProbeState::Terminal(ProbeOutcome::Missing) => (3, 0),
        SavedProbeState::Terminal(ProbeOutcome::Infeasible { reason }) => (4, reason.len()),
        SavedProbeState::Terminal(ProbeOutcome::Censored {
            reason: LimitReason::TickHorizon,
        }) => (5, 0),
        SavedProbeState::Terminal(ProbeOutcome::Censored {
            reason: LimitReason::EventBudget,
        }) => (6, 0),
        SavedProbeState::Terminal(ProbeOutcome::Failed { reason }) => (7, reason.len()),
    }
}

fn payload_valid(tag: u8, bytes: &[u8], limits: WireLimits) -> bool {
    match tag {
        1 => !bytes.is_empty() && bytes.len() <= limits.max_image_bytes,
        2 => bytes.len() == 16,
        3 | 5 | 6 => bytes.is_empty(),
        4 | 7 => {
            bytes.len() <= limits.max_reason_bytes
                && std::str::from_utf8(bytes).is_ok_and(|s| !s.trim().is_empty())
        }
        _ => false,
    }
}

pub(crate) fn encode(image: &RunImage, limits: WireLimits) -> Result<Vec<u8>, ShadowError> {
    if image.runner.version != 1 || image.runner.probes.len() > limits.max_probes {
        return Err(ShadowError::IncompatibleCheckpoint);
    }
    let mut rows = BTreeMap::new();
    let mut total = HEADER + 32;
    for row in &image.runner.probes {
        if row.spec.id.trim().is_empty()
            || row.spec.id.len() > limits.max_id_bytes
            || row.snapshot.frontier as u128 > u128::from(image.ledger_frontier)
            || rows.insert(row.spec.id.as_str(), row).is_some()
        {
            return Err(ShadowError::IncompatibleCheckpoint);
        }
        let (tag, len) = state_parts(&row.state);
        if (tag == 1 && (len == 0 || len > limits.max_image_bytes))
            || (matches!(tag, 4 | 7) && len > limits.max_reason_bytes)
        {
            return Err(ShadowError::LimitExceeded);
        }
        if matches!(&row.state, SavedProbeState::Terminal(ProbeOutcome::Failed { reason } | ProbeOutcome::Infeasible { reason }) if reason.trim().is_empty())
        {
            return Err(ShadowError::IncompatibleCheckpoint);
        }
        total = total
            .checked_add(ROW_FIXED)
            .and_then(|n| n.checked_add(row.spec.id.len()))
            .and_then(|n| n.checked_add(len))
            .ok_or(ShadowError::LimitExceeded)?;
        if total > limits.max_wire_bytes {
            return Err(ShadowError::LimitExceeded);
        }
    }
    if total > limits.max_wire_bytes {
        return Err(ShadowError::LimitExceeded);
    }
    let mut output = Vec::new();
    output
        .try_reserve_exact(total)
        .map_err(|_| ShadowError::LimitExceeded)?;
    output.extend_from_slice(MAGIC);
    output.extend_from_slice(&1u32.to_le_bytes());
    output.extend_from_slice(&image.runner.binding);
    output.extend_from_slice(&image.ledger_frontier.to_le_bytes());
    output.extend_from_slice(&(rows.len() as u64).to_le_bytes());
    for (id, row) in rows {
        output.extend_from_slice(&(id.len() as u64).to_le_bytes());
        output.extend_from_slice(id.as_bytes());
        output.extend_from_slice(&fingerprint(&row.spec, &row.snapshot)?);
        output.extend_from_slice(&row.events.to_le_bytes());
        output.extend_from_slice(&row.last_tick.to_le_bytes());
        let (tag, len) = state_parts(&row.state);
        output.push(tag);
        output.extend_from_slice(&(len as u64).to_le_bytes());
        match &row.state {
            SavedProbeState::Pending(bytes) => output.extend_from_slice(bytes),
            SavedProbeState::Terminal(ProbeOutcome::Completed { predicted }) => {
                output.extend_from_slice(&predicted.to_le_bytes())
            }
            SavedProbeState::Terminal(
                ProbeOutcome::Failed { reason } | ProbeOutcome::Infeasible { reason },
            ) => output.extend_from_slice(reason.as_bytes()),
            _ => {}
        }
    }
    let digest = Sha256::digest(&output);
    output.extend_from_slice(&digest);
    Ok(output)
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], ShadowError> {
        let end = self
            .at
            .checked_add(n)
            .ok_or(ShadowError::IncompatibleCheckpoint)?;
        let bytes = self
            .bytes
            .get(self.at..end)
            .ok_or(ShadowError::IncompatibleCheckpoint)?;
        self.at = end;
        Ok(bytes)
    }
    fn u64(&mut self) -> Result<u64, ShadowError> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn len(&mut self) -> Result<usize, ShadowError> {
        usize::try_from(self.u64()?).map_err(|_| ShadowError::LimitExceeded)
    }
}

pub(crate) fn decode(
    bytes: &[u8],
    binding: [u8; 32],
    expected_frontier: u64,
    trusted: &[(ProbeSpec, LedgerSnapshot)],
    limits: WireLimits,
) -> Result<RunImage, ShadowError> {
    if bytes.len() > limits.max_wire_bytes || trusted.len() > limits.max_probes {
        return Err(ShadowError::LimitExceeded);
    }
    let body_len = bytes
        .len()
        .checked_sub(32)
        .filter(|n| *n >= HEADER)
        .ok_or(ShadowError::IncompatibleCheckpoint)?;
    if Sha256::digest(&bytes[..body_len]).as_slice() != &bytes[body_len..] {
        return Err(ShadowError::IncompatibleCheckpoint);
    }
    let mut r = Reader {
        bytes: &bytes[..body_len],
        at: 0,
    };
    if r.take(8)? != MAGIC || r.take(4)? != 1u32.to_le_bytes() || r.take(32)? != binding {
        return Err(ShadowError::IncompatibleCheckpoint);
    }
    let frontier = r.u64()?;
    if frontier != expected_frontier {
        return Err(ShadowError::IncompatibleCheckpoint);
    }
    let count = r.len()?;
    if count != trusted.len()
        || count > limits.max_probes
        || count > (body_len - HEADER) / ROW_FIXED
    {
        return Err(ShadowError::IncompatibleCheckpoint);
    }
    let mut inventory = BTreeMap::new();
    for (spec, snapshot) in trusted {
        if snapshot.frontier as u128 > u128::from(frontier)
            || inventory
                .insert(spec.id.as_str(), (spec, snapshot))
                .is_some()
        {
            return Err(ShadowError::IncompatibleCheckpoint);
        }
    }
    // Borrowed preflight records: no native image copies until every row validates.
    let mut parsed = Vec::new();
    parsed
        .try_reserve_exact(count)
        .map_err(|_| ShadowError::LimitExceeded)?;
    let mut previous: Option<&str> = None;
    for _ in 0..count {
        let id_len = r.len()?;
        if id_len > limits.max_id_bytes {
            return Err(ShadowError::LimitExceeded);
        }
        let id = std::str::from_utf8(r.take(id_len)?)
            .map_err(|_| ShadowError::IncompatibleCheckpoint)?;
        if id.trim().is_empty() || previous.is_some_and(|p| p >= id) {
            return Err(ShadowError::IncompatibleCheckpoint);
        }
        previous = Some(id);
        let (spec, snapshot) = inventory
            .get(id)
            .ok_or(ShadowError::IncompatibleCheckpoint)?;
        if r.take(32)? != fingerprint(spec, snapshot)? {
            return Err(ShadowError::IncompatibleCheckpoint);
        }
        let events = r.u64()?;
        let last_tick = u128::from_le_bytes(r.take(16)?.try_into().unwrap());
        let tag = r.take(1)?[0];
        let len = r.len()?;
        let payload = r.take(len)?;
        if !payload_valid(tag, payload, limits) {
            return Err(ShadowError::IncompatibleCheckpoint);
        }
        parsed.push((*spec, *snapshot, events, last_tick, tag, payload));
    }
    if r.at != body_len {
        return Err(ShadowError::IncompatibleCheckpoint);
    }
    let mut probes = Vec::new();
    probes
        .try_reserve_exact(count)
        .map_err(|_| ShadowError::LimitExceeded)?;
    for (spec, snapshot, events, last_tick, tag, payload) in parsed {
        let state = match tag {
            1 => SavedProbeState::Pending(payload.to_vec()),
            2 => SavedProbeState::Terminal(ProbeOutcome::Completed {
                predicted: u128::from_le_bytes(payload.try_into().unwrap()),
            }),
            3 => SavedProbeState::Terminal(ProbeOutcome::Missing),
            4 => SavedProbeState::Terminal(ProbeOutcome::Infeasible {
                reason: std::str::from_utf8(payload).unwrap().to_owned(),
            }),
            5 => SavedProbeState::Terminal(ProbeOutcome::Censored {
                reason: LimitReason::TickHorizon,
            }),
            6 => SavedProbeState::Terminal(ProbeOutcome::Censored {
                reason: LimitReason::EventBudget,
            }),
            7 => SavedProbeState::Terminal(ProbeOutcome::Failed {
                reason: std::str::from_utf8(payload).unwrap().to_owned(),
            }),
            _ => return Err(ShadowError::IncompatibleCheckpoint),
        };
        probes.push(ProbeCheckpoint {
            spec: spec.clone(),
            snapshot: snapshot.clone(),
            events,
            last_tick,
            state,
        });
    }
    Ok(RunImage {
        ledger_frontier: frontier,
        runner: RunnerCheckpoint {
            version: 1,
            binding,
            probes,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::residuals::LogicalKey;
    use crate::seed_map::{CalibrationSeedMap, SeedPurpose};
    use crate::shadow::{ProbeBudget, ProbeInput};
    fn limits() -> WireLimits {
        WireLimits {
            max_wire_bytes: 4096,
            max_probes: 8,
            max_id_bytes: 256,
            max_image_bytes: 1024,
            max_reason_bytes: 256,
        }
    }
    fn fixture() -> (RunImage, Vec<(ProbeSpec, LedgerSnapshot)>) {
        let key = CalibrationSeedMap::new(1, "study", 9)
            .unwrap()
            .key_for("schedule", 0, "case", "task", SeedPurpose::Service)
            .unwrap();
        let spec = ProbeSpec {
            id: "p".into(),
            key: LogicalKey::default(),
            run_id: "run".into(),
            candidate_id: "candidate".into(),
            anchor_event: "a".into(),
            target_event: Some("b".into()),
            observed_target: Some(10),
            input: ProbeInput {
                target: "service_end".into(),
                seed_key: key,
                parameter_hash: [1; 32],
                adapter_hash: [2; 32],
                fidelity: "Micro".into(),
            },
            budget: ProbeBudget {
                horizon: 100,
                max_events: 20,
            },
        };
        let snapshot = LedgerSnapshot {
            frontier: 1,
            at: 0,
            anchor_event: "a".into(),
            digest: [3; 32],
            visible_events: Vec::new(),
            resources: BTreeMap::new(),
            resource_feasible: true,
            assumptions: vec!["known initial state".into()],
        };
        let trusted = vec![(spec.clone(), snapshot.clone())];
        (
            RunImage {
                ledger_frontier: 2,
                runner: RunnerCheckpoint {
                    version: 1,
                    binding: [7; 32],
                    probes: vec![ProbeCheckpoint {
                        spec,
                        snapshot,
                        events: 1,
                        last_tick: 5,
                        state: SavedProbeState::Pending(vec![1, 2, 3]),
                    }],
                },
            },
            trusted,
        )
    }
    #[test]
    fn roundtrip_all_states_and_exact_metadata() {
        let (mut image, trusted) = fixture();
        for state in [
            SavedProbeState::Pending(vec![1, 2, 3]),
            SavedProbeState::Terminal(ProbeOutcome::Completed {
                predicted: u128::MAX,
            }),
            SavedProbeState::Terminal(ProbeOutcome::Missing),
            SavedProbeState::Terminal(ProbeOutcome::Infeasible {
                reason: "capacity".into(),
            }),
            SavedProbeState::Terminal(ProbeOutcome::Censored {
                reason: LimitReason::TickHorizon,
            }),
            SavedProbeState::Terminal(ProbeOutcome::Censored {
                reason: LimitReason::EventBudget,
            }),
            SavedProbeState::Terminal(ProbeOutcome::Failed {
                reason: "domain".into(),
            }),
        ] {
            image.runner.probes[0].state = state;
            let bytes = encode(&image, limits()).unwrap();
            let restored = decode(&bytes, [7; 32], 2, &trusted, limits()).unwrap();
            assert!(restored.runner == image.runner);
            assert_eq!(encode(&restored, limits()).unwrap(), bytes);
        }
    }
    #[test]
    fn corruption_truncation_binding_and_inventory_mismatch_reject() {
        let (image, mut trusted) = fixture();
        let bytes = encode(&image, limits()).unwrap();
        for end in 0..bytes.len() {
            assert!(decode(&bytes[..end], [7; 32], 2, &trusted, limits()).is_err());
        }
        let mut corrupt = bytes.clone();
        corrupt[HEADER + 9] ^= 1;
        assert!(decode(&corrupt, [7; 32], 2, &trusted, limits()).is_err());
        assert!(decode(&bytes, [8; 32], 2, &trusted, limits()).is_err());
        assert!(decode(&bytes, [7; 32], 1, &trusted, limits()).is_err());
        trusted[0].0.budget.max_events += 1;
        assert!(decode(&bytes, [7; 32], 2, &trusted, limits()).is_err());
    }
    #[test]
    fn empty_pending_native_image_is_rejected() {
        let (mut image, _) = fixture();
        image.runner.probes[0].state = SavedProbeState::Pending(Vec::new());
        assert!(encode(&image, limits()).is_err());
    }

    #[test]
    fn wire_bounds_and_seed_snapshot_changes_reject() {
        let (image, mut trusted) = fixture();
        let bytes = encode(&image, limits()).unwrap();
        assert!(encode(
            &image,
            WireLimits {
                max_wire_bytes: bytes.len() - 1,
                ..limits()
            }
        )
        .is_err());
        assert!(decode(
            &bytes,
            [7; 32],
            2,
            &trusted,
            WireLimits {
                max_image_bytes: 2,
                ..limits()
            }
        )
        .is_err());
        trusted[0].1.assumptions.push("changed".into());
        assert!(decode(&bytes, [7; 32], 2, &trusted, limits()).is_err());
        let (_, mut trusted) = fixture();
        trusted[0].0.input.seed_key = CalibrationSeedMap::new(1, "study", 10)
            .unwrap()
            .key_for("schedule", 0, "case", "task", SeedPurpose::Service)
            .unwrap();
        assert!(decode(&bytes, [7; 32], 2, &trusted, limits()).is_err());
    }
}
