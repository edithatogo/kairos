//! Q5.2 private-kernel measurements. These are test-only and do not define a
//! production benchmark API or a canonical Track 12 scenario.
use super::{select_replacement, ClaimQueue, PriorityKey, RequestId};
use crate::preemption::{select_victim, HolderCandidate, WaitingCandidate};
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

const INPUT_SEED: u64 = 20_261_004;

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

fn measure_production_retain_rekey_cancel(n: usize, tied: bool, repeat: usize) {
    let mut base = ClaimQueue::<PriorityKey>::default();
    let mut keys = Vec::with_capacity(n);
    for index in 0..n {
        let key = priority_key(index, tied);
        base.requests.insert(key);
        keys.push(key);
    }
    // Keep occupancy fixed: clone/setup is outside each timed command. This is
    // the production retain predicate used by Command::Reprioritize and Cancel.
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
        "\"scenario\":\"production_retain_rekey_cancel_50_50\",\"n\":{n},\"priority_shape\":\"{}\",\"repeat\":{repeat},\"occupancy_before_each_op\":{n},\"commands_requested\":{commands_per_repeat},\"commands_completed\":{},\"reprioritize_commands\":{},\"cancel_commands\":{},\"derived_expected_retain_predicate_visits\":{retain_visits},\"elapsed_ns_sum\":{total_elapsed_ns},\"ops\":{commands_per_repeat},\"ns_per_command_mean\":{:.4},\"ns_per_command_p50\":{p50_ns},\"ns_per_command_p95\":{p95_ns},\"sample_elapsed_ns\":[{samples}],\"commands_per_second_mean\":{:.4},\"timed_operations\":\"production_retain_predicate_plus_reprioritize_insert_or_cancel\",\"setup_clone_excluded\":true,\"status\":\"complete\"",
        if tied { "tied" } else { "mixed" },
        command_rows.len(),
        command_rows.iter().filter(|(kind, _)| *kind == "reprioritize").count(),
        command_rows.iter().filter(|(kind, _)| *kind == "cancel").count(),
        total_elapsed_ns as f64 / command_rows.len() as f64,
        command_rows.len() as f64 * 1_000_000_000.0 / total_elapsed_ns.max(1) as f64
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
        "\"scenario\":\"production_select_victim\",\"active_capacity\":{n},\"repeat\":{repeat},\"calls\":{ops},\"holder_candidates_examined\":{candidates},\"elapsed_ns\":{elapsed_ns},\"ops\":{ops},\"ns_per_call\":{:.4},\"calls_per_second\":{:.4},\"expected_victim\":{expected},\"verified\":true",
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
        "\"scenario\":\"production_select_replacement\",\"match\":\"{}\",\"waiters_W\":{waiter_count},\"holders_H\":{holder_count},\"repeat\":{repeat},\"calls\":{calls},\"waiter_sort_items\":{},\"waiters_attempted\":{},\"holder_candidates_examined\":{},\"elapsed_ns\":{elapsed_ns},\"ops\":{calls},\"ns_per_call\":{:.4},\"calls_per_second\":{:.4},\"expected_pair\":\"{}\",\"verified\":true",
        if matched { "matched" } else { "no_match" },
        waiter_count * repetitions,
        attempts_per_call * repetitions,
        candidates_per_call * repetitions,
        elapsed_ns as f64 / repetitions as f64,
        repetitions as f64 * 1_000_000_000.0 / elapsed_ns.max(1) as f64,
        if let Some((waiter, holder)) = expected { format!("{waiter}:{holder}") } else { "none".to_string() }
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
    println!("Q52_META {{\"schema_version\":1,\"benchmark\":\"supplementary_queue_kernel\",\"seed\":{INPUT_SEED},\"priority_generator\":\"splitmix64(seed + index * golden_ratio_constant) mod 101 minus 50\",\"repeats\":{REPEATS},\"queue_operation_mix\":\"one rekey plus one cancel per initial waiter; 50 percent rekey, 50 percent cancel requests\",\"queue_sizes_are_waiters\":true,\"setup_excluded\":true,\"canonical_thresholds_unchanged\":true}}");
    for n in selected_queue_sizes() {
        for repeat in 1..=REPEATS {
            measure_legacy_fifo(n, repeat);
            measure_priority_fifo(n, true, repeat);
            measure_priority_fifo(n, false, repeat);
            measure_rekey_cancel(n, true, repeat);
            measure_rekey_cancel(n, false, repeat);
            measure_production_retain_rekey_cancel(n, true, repeat);
            measure_production_retain_rekey_cancel(n, false, repeat);
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
                }
            }
        }
    }
}
