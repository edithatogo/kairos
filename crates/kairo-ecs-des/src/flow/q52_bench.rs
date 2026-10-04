//! Q5.2 private-kernel measurements. These are test-only and do not define a
//! production benchmark API or a canonical Track 12 scenario.
use super::{
    remove_waiting_request, ClaimQueue, PriorityKey, RequestId, RequestState, ResourceRequest,
};
use crate::preemption::{
    select_ordered_replacement, select_replacement, select_victim, HolderCandidate,
    WaitingCandidate,
};
use crate::Resource;
use kairo_ecs_types::{EntityId, SimDuration, SimTime};
use std::cell::Cell;
use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::hint::black_box;
use std::rc::Rc;
use std::time::Instant;

const QUEUE_SIZES: [usize; 3] = [10, 1_000, 100_000];
const REPEATS: usize = 5;
const REPLACEMENT_HOLDER_SIZES: [usize; 3] = [1, 10, 100];
const ORDERED_SELECTOR_MAX_CALLS: usize = 1_000_000;
const ORDERED_SELECTOR_MAX_CALIBRATION_ATTEMPTS: usize = 5;
const ORDERED_SELECTOR_MIN_BATCH_NS: u128 = 1_000_000;

const INPUT_SEED: u64 = 20_261_004;

#[derive(Clone, Copy)]
enum OrderedSelectorMode {
    Matched,
    EqualPriorityNoMatch,
    IneligibleHolders,
    AllNonpreemptingWaiters,
}

impl OrderedSelectorMode {
    fn label(self) -> &'static str {
        match self {
            Self::Matched => "matched",
            Self::EqualPriorityNoMatch => "equal_priority_no_match",
            Self::IneligibleHolders => "ineligible_holders",
            Self::AllNonpreemptingWaiters => "all_nonpreempting_waiters",
        }
    }
}

fn mixed_level(seed: u64, index: usize) -> i32 {
    let mut value = seed.wrapping_add((index as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15));
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    ((value ^ (value >> 31)) % 101) as i32 - 50
}

fn selected_queue_sizes() -> Vec<usize> {
    match std::env::var("Q52_QUEUE_SIZE") {
        Ok(value) => {
            let parsed = value
                .parse::<usize>()
                .expect("Q52_QUEUE_SIZE must be an integer");
            assert!(
                QUEUE_SIZES.contains(&parsed),
                "Q52_QUEUE_SIZE must be 10, 1000, or 100000"
            );
            vec![parsed]
        }
        Err(std::env::VarError::NotPresent) => QUEUE_SIZES.to_vec(),
        Err(error) => panic!("could not read Q52_QUEUE_SIZE: {error}"),
    }
}

fn request_id(index: usize) -> RequestId {
    RequestId(EntityId::new(index as u64, 0))
}

fn priority_key(index: usize, tied: bool) -> PriorityKey {
    PriorityKey {
        level: if tied {
            0
        } else {
            mixed_level(INPUT_SEED, index)
        },
        enqueue_sequence: index as u64,
        request: request_id(index),
    }
}

fn json_row(fields: &str) {
    println!("Q52_ROW {{{fields}}}");
}

fn batch_count(n: usize) -> usize {
    (10_000usize / n.max(1)).max(1)
}

fn measure_priority_fifo(n: usize, tied: bool, repeat: usize) {
    let mut queue = ClaimQueue::<PriorityKey>::default();
    let keys: Vec<_> = (0..n).map(|index| priority_key(index, tied)).collect();
    let batches = batch_count(n);
    let started = Instant::now();
    for _ in 0..batches {
        for key in &keys {
            black_box(queue.requests.insert(*key));
        }
        for _ in 0..n {
            let key = queue.requests.iter().next().copied().expect("queue item");
            black_box(queue.requests.remove(&key));
            black_box(key);
        }
    }
    let elapsed_ns = started.elapsed().as_nanos();
    assert!(queue.requests.is_empty());
    let ops = (2 * n * batches) as u64;
    json_row(&format!(
        "\"scenario\":\"priority_fifo\",\"n\":{n},\"priority_shape\":\"{}\",\"legacy_fifo_comparable\":{},\"repeat\":{repeat},\"batches\":{batches},\"elapsed_ns\":{elapsed_ns},\"ops\":{ops},\"ns_per_op\":{:.4},\"ops_per_second\":{:.4},\"timed_operations\":\"insert_and_remove_min\",\"comparisons\":null,\"setup_excluded\":true,\"final_queue_len\":0",
        if tied { "tied" } else { "mixed" },
        tied,
        elapsed_ns as f64 / ops as f64,
        ops as f64 * 1_000_000_000.0 / elapsed_ns.max(1) as f64
    ));
}

fn measure_legacy_fifo(n: usize, repeat: usize) {
    let mut resource = Resource::new("q52-legacy-fifo", 1);
    assert!(resource.request(EntityId::new(u64::MAX, 0))); // hold its only slot
    let entities: Vec<_> = (0..n).map(|i| EntityId::new(i as u64, 0)).collect();
    let batches = batch_count(n);
    let started = Instant::now();
    for _ in 0..batches {
        for entity in &entities {
            black_box(resource.request(*entity));
        }
        for _ in 0..n {
            black_box(resource.release().expect("queued legacy waiter"));
        }
    }
    let elapsed_ns = started.elapsed().as_nanos();
    assert_eq!(resource.queue_length(), 0);
    let ops = (2 * n * batches) as u64;
    json_row(&format!(
        "\"scenario\":\"legacy_resource_fifo\",\"n\":{n},\"capacity\":1,\"comparison_class\":\"matched_fifo_primitive_only\",\"repeat\":{repeat},\"batches\":{batches},\"elapsed_ns\":{elapsed_ns},\"ops\":{ops},\"ns_per_op\":{:.4},\"ops_per_second\":{:.4},\"timed_operations\":\"request_waiter_and_release_waiter\",\"comparisons\":null,\"setup_excluded\":true,\"fifo_order_checked_in_correctness_test\":true,\"final_queue_len\":0",
        elapsed_ns as f64 / ops as f64,
        ops as f64 * 1_000_000_000.0 / elapsed_ns.max(1) as f64
    ));
}

fn measure_rekey_cancel(n: usize, tied: bool, repeat: usize) {
    let mut queue = ClaimQueue::<PriorityKey>::default();
    let mut keys = Vec::with_capacity(n);
    for index in 0..n {
        let key = priority_key(index, tied);
        queue.requests.insert(key);
        keys.push(key);
    }
    let started = Instant::now();
    for key_slot in &mut keys {
        let old = *key_slot;
        let new = PriorityKey {
            level: old.level.saturating_add(1),
            ..old
        };
        black_box(queue.requests.remove(&old));
        black_box(queue.requests.insert(new));
        *key_slot = new;
        black_box(queue.requests.remove(&new));
    }
    let elapsed_ns = started.elapsed().as_nanos();
    assert!(queue.requests.is_empty());
    let ops = (2 * n) as u64; // one rekey request and one cancel request per claim
    let tree_mutations = (3 * n) as u64; // remove+insert for rekey, then remove for cancel
    json_row(&format!(
        "\"scenario\":\"claim_queue_direct_tree_rekey_cancel_50_50\",\"n\":{n},\"priority_shape\":\"{}\",\"repeat\":{repeat},\"queue_occupancy_initial\":{n},\"queue_occupancy_final\":0,\"queue_occupancy_mean_at_mutation_start\":{:.4},\"rekeys\":{n},\"cancels\":{n},\"tree_mutations\":{tree_mutations},\"elapsed_ns\":{elapsed_ns},\"ops\":{ops},\"ns_per_request\":{:.4},\"requests_per_second\":{:.4},\"ns_per_tree_mutation\":{:.4},\"mutations_per_second\":{:.4},\"timed_operations\":\"remove_insert_rekey_then_cancel\",\"comparisons\":null,\"setup_excluded\":true,\"final_queue_len\":0",
        if tied { "tied" } else { "mixed" },
        (3.0 * n as f64 + 1.0) / 6.0,
        elapsed_ns as f64 / ops as f64,
        ops as f64 * 1_000_000_000.0 / elapsed_ns.max(1) as f64,
        elapsed_ns as f64 / tree_mutations as f64,
        tree_mutations as f64 * 1_000_000_000.0 / elapsed_ns.max(1) as f64
    ));
}

#[derive(Clone)]
struct CountedKey {
    level: i32,
    sequence: u64,
    id: u64,
    comparisons: Rc<Cell<u64>>,
}

impl PartialEq for CountedKey {
    fn eq(&self, other: &Self) -> bool {
        (self.level, self.sequence, self.id) == (other.level, other.sequence, other.id)
    }
}
impl Eq for CountedKey {}
impl PartialOrd for CountedKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for CountedKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.comparisons
            .set(self.comparisons.get().saturating_add(1));
        (self.level, self.sequence, self.id).cmp(&(other.level, other.sequence, other.id))
    }
}

fn ceil_log2(value: usize) -> u32 {
    usize::BITS - value.saturating_sub(1).leading_zeros()
}

fn measure_legacy_retain_baseline_rekey_cancel(n: usize, tied: bool, repeat: usize) {
    let mut base = ClaimQueue::<PriorityKey>::default();
    let mut keys = Vec::with_capacity(n);
    for index in 0..n {
        let key = priority_key(index, tied);
        base.requests.insert(key);
        keys.push(key);
    }
    // Historical baseline only: commit bcb11cd used retain-based queue removal.
    // Current production uses remove_waiting_request, measured separately below.
    // Keep occupancy fixed: clone/setup is outside each timed command.
    let commands_per_repeat = 100usize.min(n);
    let mut command_rows = Vec::with_capacity(commands_per_repeat);
    let mut retain_visits = 0u64;
    for sample in 0..commands_per_repeat {
        let is_reprioritize = sample % 2 == 0;
        let index = sample / 2;
        let old = keys[index];
        let mut queue = base.clone();
        let started = Instant::now();
        retain_visits = retain_visits.saturating_add(queue.requests.len() as u64);
        queue.requests.retain(|key| key.request != old.request);
        if is_reprioritize {
            let new = PriorityKey {
                level: old.level.saturating_add(1),
                ..old
            };
            queue.requests.insert(new);
            let elapsed_ns = started.elapsed().as_nanos();
            assert_eq!(queue.requests.len(), n);
            assert!(queue.requests.contains(&new));
            command_rows.push(("reprioritize", elapsed_ns));
        } else {
            let elapsed_ns = started.elapsed().as_nanos();
            assert_eq!(queue.requests.len(), n - 1);
            assert!(!queue.requests.contains(&old));
            command_rows.push(("cancel", elapsed_ns));
        }
    }
    let mut elapsed_values: Vec<u128> = command_rows.iter().map(|(_, ns)| *ns).collect();
    elapsed_values.sort_unstable();
    let total_elapsed_ns: u128 = elapsed_values.iter().sum();
    let p50_ns = elapsed_values[elapsed_values.len() / 2];
    let p95_ns = elapsed_values[(elapsed_values.len() * 95).div_ceil(100).saturating_sub(1)];
    let samples = elapsed_values
        .iter()
        .map(u128::to_string)
        .collect::<Vec<_>>()
        .join(",");
    json_row(&format!(
        "\"scenario\":\"legacy_algorithm_baseline_bcb11cd_retain_rekey_cancel_50_50\",\"algorithm_source_commit\":\"bcb11cd61bc38b4813815f574a6a051f3b9e8faf\",\"n\":{n},\"priority_shape\":\"{}\",\"repeat\":{repeat},\"occupancy_before_each_op\":{n},\"commands_requested\":{commands_per_repeat},\"commands_completed\":{},\"reprioritize_commands\":{},\"cancel_commands\":{},\"derived_expected_retain_predicate_visits\":{retain_visits},\"elapsed_ns_sum\":{total_elapsed_ns},\"ops\":{commands_per_repeat},\"ns_per_command_mean\":{:.4},\"ns_per_command_p50\":{p50_ns},\"ns_per_command_p95\":{p95_ns},\"sample_elapsed_ns\":[{samples}],\"commands_per_second_mean\":{:.4},\"timed_operations\":\"historical_retain_predicate_plus_reprioritize_insert_or_cancel\",\"setup_clone_excluded\":true,\"exploratory_timer_overhead_unadjusted\":true,\"contract_timing_evidence\":false,\"status\":\"complete\"",
        if tied { "tied" } else { "mixed" },
        command_rows.len(),
        command_rows.iter().filter(|(kind, _)| *kind == "reprioritize").count(),
        command_rows.iter().filter(|(kind, _)| *kind == "cancel").count(),
        total_elapsed_ns as f64 / command_rows.len() as f64,
        command_rows.len() as f64 * 1_000_000_000.0 / total_elapsed_ns.max(1) as f64
    ));
}

fn measure_actual_remove_waiting_rekey_cancel(n: usize, tied: bool, repeat: usize) {
    let mut base = ClaimQueue::<PriorityKey>::default();
    let mut keys = Vec::with_capacity(n);
    for index in 0..n {
        let key = priority_key(index, tied);
        base.requests.insert(key);
        keys.push(key);
    }
    // A fresh clone per command keeps occupancy fixed and clone/setup outside
    // the timer while exercising the current private helper and exact rekey.
    let commands_per_repeat = 100usize.min(n);
    let mut command_rows = Vec::with_capacity(commands_per_repeat);
    for sample in 0..commands_per_repeat {
        let is_reprioritize = sample % 2 == 0;
        let key = keys[sample / 2];
        let mut queue = base.clone();
        let mut request = ResourceRequest {
            resource: super::ResourceId(EntityId::new(0, 0)),
            owner: EntityId::new(0, 0),
            state: RequestState::Queued,
            admission_sequence: Some(key.enqueue_sequence),
            lease: None,
            priority_level: key.level,
            work: None,
            submitted_at: SimTime::from_ticks(0),
            deadline: None,
            timed: false,
            can_preempt: false,
            preemptible: None,
        };
        let started = Instant::now();
        let result = remove_waiting_request(&mut queue, key.request, &request);
        if is_reprioritize {
            let new_level = key.level.saturating_add(1);
            request.priority_level = new_level;
            let reinserted = queue.requests.insert(PriorityKey {
                level: new_level,
                enqueue_sequence: key.enqueue_sequence,
                request: key.request,
            });
            black_box(reinserted);
        } else {
            request.state = RequestState::Cancelled;
        }
        let elapsed_ns = started.elapsed().as_nanos();
        result.expect("current private waiting-removal helper");
        if is_reprioritize {
            assert_eq!(request.admission_sequence, Some(key.enqueue_sequence));
            assert_eq!(queue.requests.len(), n);
            assert!(queue.requests.contains(&PriorityKey {
                level: request.priority_level,
                enqueue_sequence: key.enqueue_sequence,
                request: key.request,
            }));
            command_rows.push(("reprioritize", elapsed_ns));
        } else {
            assert_eq!(request.state, RequestState::Cancelled);
            assert_eq!(queue.requests.len(), n - 1);
            assert!(!queue.requests.contains(&key));
            command_rows.push(("cancel", elapsed_ns));
        }
    }
    let mut elapsed_values: Vec<u128> = command_rows.iter().map(|(_, ns)| *ns).collect();
    elapsed_values.sort_unstable();
    let total_elapsed_ns: u128 = elapsed_values.iter().sum();
    let p50_ns = elapsed_values[elapsed_values.len() / 2];
    let p95_ns = elapsed_values[(elapsed_values.len() * 95).div_ceil(100).saturating_sub(1)];
    let samples = elapsed_values
        .iter()
        .map(u128::to_string)
        .collect::<Vec<_>>()
        .join(",");
    json_row(&format!(
        "\"scenario\":\"actual_remove_waiting_request_rekey_cancel_50_50\",\"n\":{n},\"priority_shape\":\"{}\",\"repeat\":{repeat},\"occupancy_before_each_op\":{n},\"commands_requested\":{commands_per_repeat},\"commands_completed\":{},\"reprioritize_commands\":{},\"cancel_commands\":{},\"removed_keys\":{},\"reinserted_keys\":{},\"elapsed_ns_sum\":{total_elapsed_ns},\"ops\":{commands_per_repeat},\"ns_per_command_mean\":{:.4},\"ns_per_command_p50\":{p50_ns},\"ns_per_command_p95\":{p95_ns},\"sample_elapsed_ns\":[{samples}],\"commands_per_second_mean\":{:.4},\"timed_operations\":\"current_private_remove_waiting_request_plus_exact_rekey_insert_or_cancel_state\",\"admission_sequence_preserved\":true,\"setup_clone_excluded\":true,\"exploratory_timer_overhead_unadjusted\":true,\"contract_timing_evidence\":false,\"status\":\"complete\"",
        if tied { "tied" } else { "mixed" },
        command_rows.len(),
        command_rows.iter().filter(|(kind, _)| *kind == "reprioritize").count(),
        command_rows.iter().filter(|(kind, _)| *kind == "cancel").count(),
        command_rows.len(),
        command_rows.iter().filter(|(kind, _)| *kind == "reprioritize").count(),
        total_elapsed_ns as f64 / command_rows.len() as f64,
        command_rows.len() as f64 * 1_000_000_000.0 / total_elapsed_ns.max(1) as f64
    ));
}

const BATCHED_ACTUAL_CYCLES: usize = 50_000;
const BATCHED_HISTORICAL_CYCLES: usize = 50_000;
const BATCHED_HISTORICAL_100K_CYCLES: usize = 100;
const ACTUAL_REMOVAL_SOURCE_COMMIT: &str = "05b7822f0c69b9d36549130e228c01689b908a13";
const HISTORICAL_RETAIN_SOURCE_COMMIT: &str = "bcb11cd61bc38b4813815f574a6a051f3b9e8faf";

fn batched_historical_cycles(n: usize) -> usize {
    if n == 100_000 {
        BATCHED_HISTORICAL_100K_CYCLES
    } else {
        BATCHED_HISTORICAL_CYCLES
    }
}

fn cycle_key_index(cycle: usize, n: usize) -> usize {
    cycle.wrapping_mul(7_919) % n
}

fn measure_batched_actual_remove_rekey_restore(n: usize, tied: bool, repeat: usize) {
    let mut queue = ClaimQueue::<PriorityKey>::default();
    let mut keys = Vec::with_capacity(n);
    for index in 0..n {
        let key = priority_key(index, tied);
        assert!(queue.requests.insert(key));
        keys.push(key);
    }
    let base = queue.clone();
    let mut request = ResourceRequest {
        resource: super::ResourceId(EntityId::new(0, 0)),
        owner: EntityId::new(0, 0),
        state: RequestState::Queued,
        admission_sequence: Some(0),
        lease: None,
        priority_level: keys[0].level,
        work: None,
        submitted_at: SimTime::from_ticks(0),
        deadline: None,
        timed: false,
        can_preempt: false,
        preemptible: None,
    };
    let cycles = BATCHED_ACTUAL_CYCLES;
    let started = Instant::now();
    for cycle in 0..cycles {
        let old = keys[cycle_key_index(cycle, n)];
        request.admission_sequence = Some(old.enqueue_sequence);
        request.priority_level = old.level;
        remove_waiting_request(&mut queue, old.request, &request)
            .expect("actual old waiting key exists");

        let changed = PriorityKey {
            level: old.level.saturating_add(1),
            ..old
        };
        request.priority_level = changed.level;
        assert!(black_box(queue.requests.insert(changed)));
        remove_waiting_request(&mut queue, old.request, &request)
            .expect("actual rekeyed waiting key exists");

        request.priority_level = old.level;
        assert!(black_box(queue.requests.insert(old)));
    }
    let elapsed_ns = started.elapsed().as_nanos();
    assert_eq!(queue.requests, base.requests);
    black_box(&queue);
    let primitive_ops = (cycles * 4) as u64;
    let restoration_insert_count = cycles as u64;
    json_row(&format!(
        "\"scenario\":\"actual_helper_batched_reversible_rekey\",\"algorithm_source_commit\":\"{ACTUAL_REMOVAL_SOURCE_COMMIT}\",\"n\":{n},\"priority_shape\":\"{}\",\"repeat\":{repeat},\"cycles\":{cycles},\"operations\":{cycles},\"primitive_queue_mutations\":{primitive_ops},\"queue_mutations_per_cycle\":4,\"restoration_insert_count\":{restoration_insert_count},\"elapsed_ns\":{elapsed_ns},\"ns_per_primitive_op\":{:.4},\"derived_expected_helper_removals\":{},\"occupancy_at_cycle_boundaries\":{n},\"final_queue_matches_initial\":true,\"setup_and_verification_excluded\":true,\"timed_operations\":\"two_current_remove_waiting_request_calls_with_success_checks_plus_rekey_insert_and_restore_insert; reversible helper cycle, not a Cancel or Reprioritize command\",\"end_to_end_speedup_claim\":false,\"status\":\"complete\"",
        if tied { "tied" } else { "mixed" },
        elapsed_ns as f64 / primitive_ops as f64,
        cycles * 2
    ));
}

fn measure_batched_historical_retain_rekey_restore(n: usize, tied: bool, repeat: usize) {
    let mut queue = ClaimQueue::<PriorityKey>::default();
    let mut keys = Vec::with_capacity(n);
    for index in 0..n {
        let key = priority_key(index, tied);
        assert!(queue.requests.insert(key));
        keys.push(key);
    }
    let base = queue.clone();
    let cycles = batched_historical_cycles(n);
    let started = Instant::now();
    for cycle in 0..cycles {
        let old = keys[cycle_key_index(cycle, n)];
        queue.requests.retain(|key| key.request != old.request);
        let changed = PriorityKey {
            level: old.level.saturating_add(1),
            ..old
        };
        assert!(black_box(queue.requests.insert(changed)));
        queue.requests.retain(|key| key.request != old.request);
        assert!(black_box(queue.requests.insert(old)));
    }
    let elapsed_ns = started.elapsed().as_nanos();
    assert_eq!(queue.requests, base.requests);
    black_box(&queue);
    let primitive_ops = (cycles * 4) as u64;
    let restoration_insert_count = cycles as u64;
    let predicate_visits = (cycles as u64).saturating_mul(2).saturating_mul(n as u64);
    json_row(&format!(
        "\"scenario\":\"historical_retain_batched_reversible_rekey\",\"algorithm_source_commit\":\"{HISTORICAL_RETAIN_SOURCE_COMMIT}\",\"n\":{n},\"priority_shape\":\"{}\",\"repeat\":{repeat},\"cycles\":{cycles},\"operations\":{cycles},\"primitive_queue_mutations\":{primitive_ops},\"queue_mutations_per_cycle\":4,\"restoration_insert_count\":{restoration_insert_count},\"elapsed_ns\":{elapsed_ns},\"ns_per_primitive_op\":{:.4},\"historical_retain_predicate_visits_exact\":{predicate_visits},\"historical_retain_predicate_visit_bound\":\"2*n*cycles\",\"occupancy_at_cycle_boundaries\":{n},\"final_queue_matches_initial\":true,\"setup_and_verification_excluded\":true,\"timed_operations\":\"two historical retain passes plus rekey insert and restore insert; reversible helper cycle, not a Cancel or Reprioritize command\",\"end_to_end_speedup_claim\":false,\"status\":\"complete\"",
        if tied { "tied" } else { "mixed" },
        elapsed_ns as f64 / primitive_ops as f64
    ));
}

// The comparison counter is interior instrumentation only; Ord reads only the
// immutable (level, sequence, id) tuple, so mutating this Cell cannot affect key order.
#[allow(clippy::mutable_key_type)]
fn counted_comparisons(n: usize, tied: bool) -> (u64, u64) {
    let comparisons = Rc::new(Cell::new(0));
    let mut queue = BTreeSet::new();
    let mut keys = Vec::with_capacity(n);
    for index in 0..n {
        let key = CountedKey {
            level: if tied {
                0
            } else {
                mixed_level(INPUT_SEED, index)
            },
            sequence: index as u64,
            id: index as u64,
            comparisons: comparisons.clone(),
        };
        queue.insert(key.clone());
        keys.push(key);
    }
    comparisons.set(0); // setup excluded from counted operation totals
    for key_slot in &mut keys {
        let old = &*key_slot;
        assert!(queue.remove(old));
        let new = CountedKey {
            level: old.level.saturating_add(1),
            sequence: old.sequence,
            id: old.id,
            comparisons: comparisons.clone(),
        };
        assert!(queue.insert(new.clone()));
        *key_slot = new.clone();
        assert!(queue.remove(&new));
    }
    assert!(queue.is_empty());
    (comparisons.get(), (3 * n) as u64)
}

fn eligible_holders(n: usize, tied: bool) -> Vec<HolderCandidate<u64>> {
    (0..n)
        .map(|index| HolderCandidate {
            id: index as u64,
            priority_level: if tied {
                20
            } else {
                20 + (mixed_level(INPUT_SEED, index).unsigned_abs() % 17) as i32
            },
            original_admission_sequence: index as u64,
            remaining: SimDuration::from_ticks(5),
            completion_at: Some(SimTime::from_ticks(10)),
            timed: true,
            preemptible: true,
        })
        .collect()
}

fn measure_select_victim(n: usize, repeat: usize) {
    let holders = eligible_holders(n, false);
    let expected = holders
        .iter()
        .max_by_key(|h| (h.priority_level, h.original_admission_sequence, h.id))
        .map(|h| h.id)
        .unwrap();
    let repetitions = 1_000usize;
    let started = Instant::now();
    let mut last = None;
    for _ in 0..repetitions {
        last = black_box(select_victim(
            black_box(1),
            black_box(true),
            black_box(SimTime::from_ticks(0)),
            black_box(&holders[..]),
        ));
    }
    let elapsed_ns = started.elapsed().as_nanos();
    assert_eq!(last, Some(expected));
    let ops = repetitions as u64;
    let candidates = (repetitions * n) as u64;
    json_row(&format!(
        "\"scenario\":\"historical_victim_reference\",\"active_capacity\":{n},\"repeat\":{repeat},\"calls\":{ops},\"holder_candidates_examined\":{candidates},\"elapsed_ns\":{elapsed_ns},\"ops\":{ops},\"ns_per_call\":{:.4},\"calls_per_second\":{:.4},\"expected_victim\":{expected},\"verified\":true",
        elapsed_ns as f64 / repetitions as f64,
        repetitions as f64 * 1_000_000_000.0 / elapsed_ns.max(1) as f64
    ));
}

fn replacement_inputs(
    waiter_count: usize,
    holder_count: usize,
    matched: bool,
) -> (Vec<WaitingCandidate<u64>>, Vec<HolderCandidate<u64>>) {
    let waiters = (0..waiter_count)
        .map(|index| WaitingCandidate {
            id: index as u64,
            priority_level: if matched { 10 } else { 0 },
            original_admission_sequence: index as u64,
            can_preempt: true,
        })
        .collect();
    let holders = (0..holder_count)
        .map(|index| HolderCandidate {
            id: index as u64,
            priority_level: if matched { 11 } else { 0 },
            original_admission_sequence: index as u64,
            remaining: SimDuration::from_ticks(5),
            completion_at: Some(SimTime::from_ticks(10)),
            timed: true,
            preemptible: true,
        })
        .collect();
    (waiters, holders)
}

fn measure_select_replacement(
    waiter_count: usize,
    holder_count: usize,
    matched: bool,
    repeat: usize,
) {
    let (waiters, holders) = replacement_inputs(waiter_count, holder_count, matched);
    let expected = if matched {
        Some((0, holder_count as u64 - 1))
    } else {
        None
    };
    let sort_work = waiter_count.saturating_mul(ceil_log2(waiter_count + 1) as usize);
    let scan_work = if matched {
        holder_count
    } else {
        waiter_count.saturating_mul(holder_count)
    };
    let per_call = sort_work.saturating_add(scan_work);
    let target_candidates = 1_000_000usize;
    let repetitions = (target_candidates / per_call.max(1)).clamp(1, 1_000);
    let mut observed = None;
    let started = Instant::now();
    for _ in 0..repetitions {
        observed = black_box(select_replacement(
            black_box(SimTime::from_ticks(0)),
            black_box(&waiters[..]),
            black_box(&holders[..]),
        ));
    }
    let elapsed_ns = started.elapsed().as_nanos();
    assert_eq!(observed, expected);
    let attempts_per_call = if matched { 1 } else { waiter_count };
    let candidates_per_call = if matched {
        holder_count
    } else {
        waiter_count * holder_count
    };
    let calls = repetitions as u64;
    json_row(&format!(
        "\"scenario\":\"historical_sorting_reference\",\"implementation_label\":\"historical_sorting_reference\",\"match\":\"{}\",\"waiters_W\":{waiter_count},\"holders_H\":{holder_count},\"repeat\":{repeat},\"calls\":{calls},\"waiter_sort_items\":{},\"waiters_attempted\":{},\"holder_candidates_examined\":{},\"elapsed_ns\":{elapsed_ns},\"ops\":{calls},\"ns_per_call\":{:.4},\"calls_per_second\":{:.4},\"expected_pair\":\"{}\",\"verified\":true",
        if matched { "matched" } else { "no_match" },
        waiter_count * repetitions,
        attempts_per_call * repetitions,
        candidates_per_call * repetitions,
        elapsed_ns as f64 / repetitions as f64,
        repetitions as f64 * 1_000_000_000.0 / elapsed_ns.max(1) as f64,
        if let Some((waiter, holder)) = expected { format!("{waiter}:{holder}") } else { "none".to_string() }
    ));
}

fn ordered_selector_inputs(
    waiter_count: usize,
    holder_count: usize,
    mode: OrderedSelectorMode,
) -> (Vec<WaitingCandidate<u64>>, Vec<HolderCandidate<u64>>) {
    let waiters = (0..waiter_count)
        .map(|index| WaitingCandidate {
            id: index as u64,
            priority_level: 10,
            original_admission_sequence: index as u64,
            can_preempt: !matches!(mode, OrderedSelectorMode::AllNonpreemptingWaiters),
        })
        .collect();
    let holders = (0..holder_count)
        .map(|index| {
            let mut candidate = HolderCandidate {
                id: index as u64,
                priority_level: match mode {
                    OrderedSelectorMode::Matched
                    | OrderedSelectorMode::IneligibleHolders
                    | OrderedSelectorMode::AllNonpreemptingWaiters => 11,
                    OrderedSelectorMode::EqualPriorityNoMatch => 10,
                },
                original_admission_sequence: index as u64,
                remaining: SimDuration::from_ticks(5),
                completion_at: Some(SimTime::from_ticks(10)),
                timed: true,
                preemptible: true,
            };
            if matches!(mode, OrderedSelectorMode::IneligibleHolders) {
                match index % 5 {
                    0 => candidate.timed = false,
                    1 => candidate.preemptible = false,
                    2 => candidate.remaining = SimDuration::ZERO,
                    3 => candidate.completion_at = Some(SimTime::from_ticks(0)),
                    _ => candidate.completion_at = None,
                }
            }
            candidate
        })
        .collect();
    (waiters, holders)
}

fn measure_ordered_selector_kernel(
    waiter_count: usize,
    holder_count: usize,
    mode: OrderedSelectorMode,
    repeat: usize,
) {
    const NOW: SimTime = SimTime::from_ticks(0);
    let (mut waiters, holders) = ordered_selector_inputs(waiter_count, holder_count, mode);
    // Scramble deterministically, then prepare the ordered production input
    // outside all timed batches.
    waiters.reverse();
    let expected = select_replacement(NOW, &waiters, &holders);
    let mut ordered_waiters = waiters;
    ordered_waiters.sort_by_key(|candidate| {
        (
            candidate.priority_level,
            candidate.original_admission_sequence,
            candidate.id,
        )
    });

    let yielded_waiters = Cell::new(0usize);
    let probe = ordered_waiters
        .iter()
        .copied()
        .inspect(|_| yielded_waiters.set(yielded_waiters.get() + 1));
    assert_eq!(
        select_ordered_replacement(NOW, probe, &holders),
        expected,
        "ordered selector disagrees with sorting reference: mode={}, W={waiter_count}, H={holder_count}",
        mode.label()
    );
    let yielded_per_call = yielded_waiters.get();
    let initial_calls = (1_000_000usize / holder_count.saturating_add(yielded_per_call).max(1))
        .clamp(1, ORDERED_SELECTOR_MAX_CALLS);

    let mut attempts = Vec::with_capacity(ORDERED_SELECTOR_MAX_CALIBRATION_ATTEMPTS);
    let mut calls = initial_calls;
    let mut stop_reason = "attempt_cap";
    for attempt in 1..=ORDERED_SELECTOR_MAX_CALIBRATION_ATTEMPTS {
        let started = Instant::now();
        let mut observed = None;
        for _ in 0..calls {
            observed = black_box(select_ordered_replacement(
                black_box(NOW),
                black_box(ordered_waiters.iter().copied()),
                black_box(&holders[..]),
            ));
        }
        let elapsed_ns = started.elapsed().as_nanos();
        assert_eq!(observed, expected);
        attempts.push((attempt, calls, elapsed_ns));

        if elapsed_ns >= ORDERED_SELECTOR_MIN_BATCH_NS {
            stop_reason = "minimum_batch_duration_reached";
            break;
        }
        if calls >= ORDERED_SELECTOR_MAX_CALLS {
            stop_reason = "maximum_calls_reached_submillisecond";
            break;
        }
        if attempt == ORDERED_SELECTOR_MAX_CALIBRATION_ATTEMPTS {
            stop_reason = "calibration_attempt_cap_submillisecond";
            break;
        }
        let next_calls = calls.saturating_mul(2).min(ORDERED_SELECTOR_MAX_CALLS);
        if next_calls <= calls {
            stop_reason = "call_count_did_not_increase_submillisecond";
            break;
        }
        calls = next_calls;
    }

    let (final_attempt, final_calls, final_elapsed_ns) =
        *attempts.last().expect("at least one calibration attempt");
    let attempt_json = attempts
        .iter()
        .map(|(attempt, calls, elapsed_ns)| {
            format!("{{\"attempt\":{attempt},\"calls\":{calls},\"elapsed_ns\":{elapsed_ns}}}")
        })
        .collect::<Vec<_>>()
        .join(",");
    let precision = if final_elapsed_ns >= ORDERED_SELECTOR_MIN_BATCH_NS {
        "minimum_batch_duration_reached"
    } else {
        "exploratory_submillisecond_no_precision_claim"
    };
    let expected_pair = expected
        .map(|(waiter, holder)| format!("{waiter}:{holder}"))
        .unwrap_or_else(|| "none".to_owned());
    json_row(&format!(
        "\"scenario\":\"production_ordered_selector_kernel\",\"scope_label\":\"ordered helper only; excludes Flow index maintenance and holder projection\",\"mode\":\"{}\",\"waiters_W\":{waiter_count},\"holders_H\":{holder_count},\"repeat\":{repeat},\"calls\":{final_calls},\"elapsed_ns\":{final_elapsed_ns},\"ns_per_call\":{:.4},\"yielded_waiters_per_call_untimed\":{yielded_per_call},\"expected_pair\":\"{expected_pair}\",\"verified\":true,\"attempt_count\":{},\"attempts\":[{attempt_json}],\"calibration_stop_reason\":\"{stop_reason}\",\"precision_status\":\"{precision}\",\"max_calls\":{ORDERED_SELECTOR_MAX_CALLS},\"max_calibration_attempts\":{ORDERED_SELECTOR_MAX_CALIBRATION_ATTEMPTS},\"min_batch_ns\":{ORDERED_SELECTOR_MIN_BATCH_NS},\"initial_call_formula\":\"clamp(1000000 / max(1,H+yielded_waiters),1,1000000)\",\"timed_operations\":\"loop and ordered helper invocations with black-boxed result\",\"waiters_sorted_outside_timer\":true,\"fixture_creation_and_holder_construction_excluded\":true,\"production_flow_integrated\":false,\"end_to_end_speedup_claim\":false",
        mode.label(),
        final_elapsed_ns as f64 / final_calls as f64,
        final_attempt
    ));
}

#[test]
fn q52_kernel_correctness_small() {
    let mut queue = ClaimQueue::<PriorityKey>::default();
    let first = priority_key(0, true);
    let second = priority_key(1, true);
    let third = priority_key(2, true);
    assert!(queue.requests.insert(first));
    assert!(queue.requests.insert(second));
    assert!(queue.requests.insert(third));
    let changed = PriorityKey { level: -1, ..third };
    assert!(queue.requests.remove(&third));
    assert!(queue.requests.insert(changed));
    assert_eq!(queue.requests.iter().next().copied(), Some(changed));
    assert!(queue.requests.remove(&changed));
    assert_eq!(queue.requests.iter().next().copied(), Some(first));
    assert!(queue.requests.remove(&first));
    assert!(queue.requests.remove(&second));
    assert!(queue.requests.is_empty());

    // Exercise the current private removal helper and preserve the original
    // admission sequence when the waiting key is reprioritized.
    let mut helper_queue = ClaimQueue::<PriorityKey>::default();
    let helper_old = PriorityKey {
        level: 7,
        enqueue_sequence: 44,
        request: request_id(44),
    };
    assert!(helper_queue.requests.insert(helper_old));
    let mut helper_request = ResourceRequest {
        resource: super::ResourceId(EntityId::new(0, 0)),
        owner: EntityId::new(0, 0),
        state: RequestState::Queued,
        admission_sequence: Some(44),
        lease: None,
        priority_level: 7,
        work: None,
        submitted_at: SimTime::from_ticks(0),
        deadline: None,
        timed: false,
        can_preempt: false,
        preemptible: None,
    };
    assert!(remove_waiting_request(&mut helper_queue, helper_old.request, &helper_request).is_ok());
    helper_request.priority_level = -2;
    let helper_new = PriorityKey {
        level: helper_request.priority_level,
        enqueue_sequence: helper_request.admission_sequence.unwrap(),
        request: helper_old.request,
    };
    assert!(helper_queue.requests.insert(helper_new));
    assert_eq!(helper_new.enqueue_sequence, helper_old.enqueue_sequence);
    assert!(helper_queue.requests.contains(&helper_new));
    assert!(remove_waiting_request(&mut helper_queue, helper_old.request, &helper_request).is_ok());
    assert!(helper_queue.requests.is_empty());

    let mut cycle_queue = ClaimQueue::<PriorityKey>::default();
    for index in 0..3 {
        assert!(cycle_queue.requests.insert(priority_key(index, false)));
    }
    let cycle_base = cycle_queue.clone();
    let cycle_old = priority_key(1, false);
    let mut cycle_request = ResourceRequest {
        resource: super::ResourceId(EntityId::new(0, 0)),
        owner: EntityId::new(0, 0),
        state: RequestState::Queued,
        admission_sequence: Some(cycle_old.enqueue_sequence),
        lease: None,
        priority_level: cycle_old.level,
        work: None,
        submitted_at: SimTime::from_ticks(0),
        deadline: None,
        timed: false,
        can_preempt: false,
        preemptible: None,
    };
    remove_waiting_request(&mut cycle_queue, cycle_old.request, &cycle_request).unwrap();
    let cycle_new = PriorityKey {
        level: cycle_old.level.saturating_add(1),
        ..cycle_old
    };
    cycle_request.priority_level = cycle_new.level;
    assert!(cycle_queue.requests.insert(cycle_new));
    remove_waiting_request(&mut cycle_queue, cycle_old.request, &cycle_request).unwrap();
    cycle_request.priority_level = cycle_old.level;
    assert!(cycle_queue.requests.insert(cycle_old));
    assert_eq!(cycle_request.priority_level, cycle_old.level);
    assert_eq!(cycle_queue.requests, cycle_base.requests);

    let mut mixed = ClaimQueue::<PriorityKey>::default();
    let mixed_keys = [
        PriorityKey {
            level: 5,
            enqueue_sequence: 0,
            request: request_id(0),
        },
        PriorityKey {
            level: -1,
            enqueue_sequence: 1,
            request: request_id(1),
        },
        PriorityKey {
            level: 5,
            enqueue_sequence: 2,
            request: request_id(2),
        },
    ];
    for key in mixed_keys {
        mixed.requests.insert(key);
    }
    let mixed_order: Vec<_> = (0..3)
        .map(|_| {
            let key = *mixed.requests.iter().next().expect("mixed priority waiter");
            mixed.requests.remove(&key);
            key.request.entity_id()
        })
        .collect();
    assert_eq!(
        mixed_order,
        vec![
            EntityId::new(1, 0),
            EntityId::new(0, 0),
            EntityId::new(2, 0)
        ]
    );

    let holders = eligible_holders(3, true);
    assert_eq!(
        select_victim(1, true, SimTime::from_ticks(0), &holders),
        Some(2)
    );
    let (waiters, no_match_holders) = replacement_inputs(3, 2, false);
    assert_eq!(
        select_replacement(SimTime::from_ticks(0), &waiters, &no_match_holders),
        None
    );
    let (waiters, match_holders) = replacement_inputs(3, 2, true);
    assert_eq!(
        select_replacement(SimTime::from_ticks(0), &waiters, &match_holders),
        Some((0, 1))
    );

    // Legacy FIFO and tied PriorityKey queues expose the same primitive order.
    let mut legacy = Resource::new("q52-correctness", 1);
    assert!(legacy.request(EntityId::new(u64::MAX, 0)));
    let ids: Vec<_> = (0..4).map(|i| EntityId::new(i, 0)).collect();
    for id in &ids {
        assert!(!legacy.request(*id));
    }
    let old_order: Vec<_> = (0..4)
        .map(|_| legacy.release().expect("legacy waiter"))
        .collect();
    let mut tied = ClaimQueue::<PriorityKey>::default();
    for index in 0..4 {
        tied.requests.insert(priority_key(index, true));
    }
    let new_order: Vec<_> = (0..4)
        .map(|_| {
            let key = *tied.requests.iter().next().expect("priority waiter");
            tied.requests.remove(&key);
            key.request.entity_id()
        })
        .collect();
    assert_eq!(old_order, ids);
    assert_eq!(new_order, ids);

    for n in [10, 1_000] {
        let (comparisons, ops) = counted_comparisons(n, false);
        let bound = ops * (8 * u64::from(ceil_log2(n + 1)) + 16);
        assert!(
            comparisons <= bound,
            "n={n}, comparisons={comparisons}, bound={bound}"
        );
    }
}

#[test]
#[ignore = "explicit Q5.2 release measurement; emits supplementary raw rows"]
fn benchmark_q52_measurements() {
    println!("Q52_META {{\"schema_version\":1,\"benchmark\":\"supplementary_queue_kernel\",\"seed\":{INPUT_SEED},\"priority_generator\":\"splitmix64(seed + index * golden_ratio_constant) mod 101 minus 50\",\"repeats\":{REPEATS},\"queue_operation_mix\":\"one rekey plus one cancel per initial waiter; 50 percent rekey, 50 percent cancel requests\",\"queue_sizes_are_waiters\":true,\"single_operation_removal_rows_are_exploratory\":true,\"batched_actual_cycles_per_repeat\":50000,\"batched_historical_cycles_n_100000\":100,\"ordered_selector_rows\":true,\"ordered_selector_modes\":[\"matched\",\"equal_priority_no_match\",\"ineligible_holders\",\"all_nonpreempting_waiters\"],\"ordered_selector_initial_calls\":\"clamp(1000000 / max(1,H+yielded_waiters),1,1000000)\",\"ordered_selector_max_calibration_attempts\":{ORDERED_SELECTOR_MAX_CALIBRATION_ATTEMPTS},\"ordered_selector_min_batch_ns\":{ORDERED_SELECTOR_MIN_BATCH_NS},\"ordered_selector_sorted_waiter_fixture_excluded\":true,\"setup_excluded\":true,\"canonical_thresholds_unchanged\":true}}");
    for n in selected_queue_sizes() {
        for repeat in 1..=REPEATS {
            measure_legacy_fifo(n, repeat);
            measure_priority_fifo(n, true, repeat);
            measure_priority_fifo(n, false, repeat);
            measure_rekey_cancel(n, true, repeat);
            measure_rekey_cancel(n, false, repeat);
            measure_legacy_retain_baseline_rekey_cancel(n, true, repeat);
            measure_legacy_retain_baseline_rekey_cancel(n, false, repeat);
            measure_actual_remove_waiting_rekey_cancel(n, true, repeat);
            measure_actual_remove_waiting_rekey_cancel(n, false, repeat);
            measure_batched_actual_remove_rekey_restore(n, true, repeat);
            measure_batched_actual_remove_rekey_restore(n, false, repeat);
            measure_batched_historical_retain_rekey_restore(n, true, repeat);
            measure_batched_historical_retain_rekey_restore(n, false, repeat);
        }
        for tied in [true, false] {
            let (comparisons, ops) = counted_comparisons(n, tied);
            let bound = ops * (8 * u64::from(ceil_log2(n + 1)) + 16);
            assert!(
                comparisons <= bound,
                "counted Ord comparison bound exceeded: n={n}"
            );
            json_row(&format!(
                "\"scenario\":\"counted_ord_structural_comparisons\",\"n\":{n},\"priority_shape\":\"{}\",\"operations\":{ops},\"comparisons\":{comparisons},\"comparisons_per_operation\":{:.6},\"ceil_log2_n_plus_1\":{},\"structural_bound\":{bound},\"elapsed_ns\":null,\"timing_claim\":false,\"setup_excluded\":true",
                if tied { "tied" } else { "mixed" },
                comparisons as f64 / ops.max(1) as f64,
                ceil_log2(n + 1)
            ));
        }
    }
    for active_capacity in [1, 10, 100, 1_000] {
        for repeat in 1..=REPEATS {
            measure_select_victim(active_capacity, repeat);
        }
    }
    for waiter_count in selected_queue_sizes() {
        for holder_count in REPLACEMENT_HOLDER_SIZES {
            for matched in [false, true] {
                for repeat in 1..=REPEATS {
                    measure_select_replacement(waiter_count, holder_count, matched, repeat);
                    measure_ordered_selector_kernel(
                        waiter_count,
                        holder_count,
                        if matched {
                            OrderedSelectorMode::Matched
                        } else {
                            OrderedSelectorMode::EqualPriorityNoMatch
                        },
                        repeat,
                    );
                }
            }
            for mode in [
                OrderedSelectorMode::IneligibleHolders,
                OrderedSelectorMode::AllNonpreemptingWaiters,
            ] {
                for repeat in 1..=REPEATS {
                    measure_ordered_selector_kernel(waiter_count, holder_count, mode, repeat);
                }
            }
        }
    }
}
