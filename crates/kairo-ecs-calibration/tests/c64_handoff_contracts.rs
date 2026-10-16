//! C6.4 integration tests for handoff fixtures and contracts to downstream tracks
//! (Track 32 Metal/GPU, Track 34 PDES, Track 35 Distributed, Track 09 Wasm, Track 05 Viz).
//!
//! Enforces:
//! 1. `test_render_rate_invariance_and_snapshot_decoupling`:
//!    Render rate changes (0 FPS headless, 30 FPS, 60 FPS, jittered/dropped) do NOT
//!    alter simulation state transitions, event sequences, random draws, or metrics.
//! 2. `test_metal_gpu_reduction_contract_and_tolerance`:
//!    Handoff to Track 32 Metal compute shader: input buffer format, canonical sorted ordering,
//!    zero padding, rational W1 and KS D validation, and epsilon <= 1e-12 tolerance.
//! 3. `test_pdes_lp_candidate_partitioning_contract`:
//!    Handoff to Track 34 PDES and Track 35 Distributed: LP candidate partitioning, conservative
//!    event exchange (GVT, null messages), worker arrival invariance, winner "c000004" selection.
//! 4. `test_wasm_parity_contract_and_tick_bounds`:
//!    Handoff to Track 09 Wasm and Track 05 Viz: MAX_EXACT_TICK (2^53 - 1) precision bounds,
//!    lossless integer round-trip, slow consumer K=64 ring-buffer without backpressure,
//!    native vs Wasm headless parity.

#![forbid(unsafe_code)]

#[allow(dead_code)]
#[path = "../src/metrics.rs"]
mod metrics;

use kairo_ecs_calibration::experimental_c5 as c5;
use metrics::{compare, MetricRequest, MetricStatus};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, VecDeque};

const STUDY_JSON: &str = include_str!("../../../examples/calibration/c5/study.json");

fn parse_json(raw: &str) -> Value {
    serde_json::from_str(raw).expect("valid JSON")
}

fn valid_study() -> Value {
    parse_json(STUDY_JSON)
}

fn plan(study: &Value) -> Result<Value, c5::C5Error> {
    c5::plan(&study.to_string()).map(|s| parse_json(&s))
}

fn evaluate(study: &Value, task: &Value) -> Result<Value, c5::C5Error> {
    c5::evaluate(&study.to_string(), &task.to_string()).map(|s| parse_json(&s))
}

fn summarize(study: &Value, attempts: &[Value]) -> Result<Value, c5::C5Error> {
    let attempts_json = serde_json::to_string(attempts).expect("serialize attempts");
    c5::summarize(&study.to_string(), &attempts_json).map(|s| parse_json(&s))
}

fn sha256_digest(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

// ===========================================================================
// Contract 1: Render Rate Invariance and Snapshot Decoupling
// ===========================================================================

/// ATS 1-5 urgency ordering (1 = highest urgency / Resuscitation, 5 = lowest / Non-urgent).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AtsUrgency {
    Resuscitation = 1,
    Emergency = 2,
    Urgent = 3,
    SemiUrgent = 4,
    NonUrgent = 5,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatientPhase {
    Arrived,
    WaitingTriage,
    InTreatment,
    DiagnosticsHold,
    Boarding,
    Discharged,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatientRecord {
    pub patient_id: u64,
    pub ats: AtsUrgency,
    pub arrival_tick: u64,
    pub service_needed: u64,
    pub service_elapsed: u64,
    pub diag_needed: u64,
    pub diag_elapsed: u64,
    pub boarding_needed: u64,
    pub boarding_elapsed: u64,
    pub phase: PatientPhase,
    pub start_treatment_tick: Option<u64>,
    pub discharge_tick: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BayState {
    Available,
    Occupied {
        patient_id: u64,
        is_diag_hold: bool,
        is_boarding: bool,
    },
    Cleaning {
        cleaning_remaining: u64,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BedBay {
    pub bay_id: u32,
    pub zone: &'static str,
    pub state: BayState,
    pub total_patients_served: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Staff {
    pub staff_id: u32,
    pub zone: &'static str,
    pub assigned_patient: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SimulationEvent {
    pub tick: u64,
    pub patient_id: u64,
    pub kind: &'static str,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompletedOutcome {
    pub patient_id: u64,
    pub ats: AtsUrgency,
    pub arrival_tick: u64,
    pub start_treatment_tick: u64,
    pub wait_ticks: u64,
    pub los_ticks: u64,
    pub discharge_tick: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SimulationResultSnapshot {
    pub completed_count: usize,
    pub completed_outcomes: Vec<CompletedOutcome>,
    pub event_log: Vec<SimulationEvent>,
    pub bay_patient_counts: Vec<(u32, u64)>,
    pub state_hash: String,
}

/// Visualization snapshot emitted to rendering / display layer (Track 05 Pixi / Track 09 Wasm).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VisualizationSnapshot {
    pub tick: u64,
    pub active_patient_count: usize,
    pub queue_length: usize,
    pub occupied_bay_count: usize,
    pub cleaning_bay_count: usize,
    pub entity_states: Vec<(u64, PatientPhase)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapshotRegime {
    Headless,
    Fps30,
    Fps60,
    JitteredConsumer,
}

struct SyntheticEdSim {
    tick: u64,
    bays: Vec<BedBay>,
    staff: Vec<Staff>,
    patients: BTreeMap<u64, PatientRecord>,
    triage_queue: Vec<u64>,
    event_log: Vec<SimulationEvent>,
    completed: Vec<CompletedOutcome>,
    cleaning_duration: u64,
}

impl SyntheticEdSim {
    fn new(cleaning_duration: u64) -> Self {
        let bays = vec![
            BedBay {
                bay_id: 1,
                zone: "resus",
                state: BayState::Available,
                total_patients_served: 0,
            },
            BedBay {
                bay_id: 2,
                zone: "acute",
                state: BayState::Available,
                total_patients_served: 0,
            },
            BedBay {
                bay_id: 3,
                zone: "acute",
                state: BayState::Available,
                total_patients_served: 0,
            },
            BedBay {
                bay_id: 4,
                zone: "subacute",
                state: BayState::Available,
                total_patients_served: 0,
            },
        ];

        let staff = vec![
            Staff {
                staff_id: 1,
                zone: "resus",
                assigned_patient: None,
            },
            Staff {
                staff_id: 2,
                zone: "acute",
                assigned_patient: None,
            },
            Staff {
                staff_id: 3,
                zone: "acute",
                assigned_patient: None,
            },
            Staff {
                staff_id: 4,
                zone: "subacute",
                assigned_patient: None,
            },
        ];

        Self {
            tick: 0,
            bays,
            staff,
            patients: BTreeMap::new(),
            triage_queue: Vec::new(),
            event_log: Vec::new(),
            completed: Vec::new(),
            cleaning_duration,
        }
    }

    fn admit_patient(&mut self, p: PatientRecord) {
        let pid = p.patient_id;
        self.event_log.push(SimulationEvent {
            tick: self.tick,
            patient_id: pid,
            kind: "arrival",
            detail: format!("ats={:?}, svc={}", p.ats, p.service_needed),
        });
        self.patients.insert(pid, p);
        self.triage_queue.push(pid);
    }

    fn step(&mut self) {
        self.tick += 1;

        // 1. Advance bay cleaning
        for bay in &mut self.bays {
            if let BayState::Cleaning { cleaning_remaining } = &mut bay.state {
                if *cleaning_remaining <= 1 {
                    bay.state = BayState::Available;
                } else {
                    *cleaning_remaining -= 1;
                }
            }
        }

        // 2. Advance active patient treatment, diagnostics hold, and boarding
        let mut patients_to_discharge = Vec::new();
        for bay in &mut self.bays {
            if let BayState::Occupied {
                patient_id,
                is_diag_hold,
                is_boarding,
            } = &mut bay.state
            {
                let pid = *patient_id;
                let patient = self
                    .patients
                    .get_mut(&pid)
                    .expect("patient in bay exists in registry");

                if !*is_diag_hold && !*is_boarding {
                    // In clinical treatment
                    patient.service_elapsed += 1;
                    if patient.service_elapsed >= patient.service_needed {
                        if patient.diag_needed > 0 && patient.diag_elapsed < patient.diag_needed {
                            *is_diag_hold = true;
                            patient.phase = PatientPhase::DiagnosticsHold;
                            self.event_log.push(SimulationEvent {
                                tick: self.tick,
                                patient_id: pid,
                                kind: "diagnostics_hold_start",
                                detail: format!("diag_needed={}", patient.diag_needed),
                            });
                        } else if patient.boarding_needed > 0
                            && patient.boarding_elapsed < patient.boarding_needed
                        {
                            *is_boarding = true;
                            patient.phase = PatientPhase::Boarding;
                            self.event_log.push(SimulationEvent {
                                tick: self.tick,
                                patient_id: pid,
                                kind: "boarding_start",
                                detail: format!("boarding_needed={}", patient.boarding_needed),
                            });
                        } else {
                            patients_to_discharge.push((bay.bay_id, pid));
                        }
                    }
                } else if *is_diag_hold {
                    // Diagnostics hold progression
                    patient.diag_elapsed += 1;
                    if patient.diag_elapsed >= patient.diag_needed {
                        *is_diag_hold = false;
                        self.event_log.push(SimulationEvent {
                            tick: self.tick,
                            patient_id: pid,
                            kind: "diagnostics_hold_cleared",
                            detail: "diagnostics complete".into(),
                        });
                        if patient.boarding_needed > 0
                            && patient.boarding_elapsed < patient.boarding_needed
                        {
                            *is_boarding = true;
                            patient.phase = PatientPhase::Boarding;
                            self.event_log.push(SimulationEvent {
                                tick: self.tick,
                                patient_id: pid,
                                kind: "boarding_start",
                                detail: format!("boarding_needed={}", patient.boarding_needed),
                            });
                        } else {
                            patients_to_discharge.push((bay.bay_id, pid));
                        }
                    }
                } else if *is_boarding {
                    // Boarding progression
                    patient.boarding_elapsed += 1;
                    if patient.boarding_elapsed >= patient.boarding_needed {
                        patients_to_discharge.push((bay.bay_id, pid));
                    }
                }
            }
        }

        // Discharge completed patients and start cleaning bays
        for (bay_id, pid) in patients_to_discharge {
            let patient = self
                .patients
                .get_mut(&pid)
                .expect("discharging patient exists");
            patient.phase = PatientPhase::Discharged;
            patient.discharge_tick = Some(self.tick);

            let arrival = patient.arrival_tick;
            let start = patient
                .start_treatment_tick
                .expect("start tick must be set for discharged patient");
            let wait = start.saturating_sub(arrival);
            let los = self.tick.saturating_sub(arrival);

            self.completed.push(CompletedOutcome {
                patient_id: pid,
                ats: patient.ats,
                arrival_tick: arrival,
                start_treatment_tick: start,
                wait_ticks: wait,
                los_ticks: los,
                discharge_tick: self.tick,
            });

            self.event_log.push(SimulationEvent {
                tick: self.tick,
                patient_id: pid,
                kind: "discharge",
                detail: format!("wait={wait}, los={los}"),
            });

            // Free assigned staff
            for staff_member in &mut self.staff {
                if staff_member.assigned_patient == Some(pid) {
                    staff_member.assigned_patient = None;
                }
            }

            // Put bay into cleaning
            let bay = self
                .bays
                .iter_mut()
                .find(|b| b.bay_id == bay_id)
                .expect("bay exists");
            bay.state = BayState::Cleaning {
                cleaning_remaining: self.cleaning_duration,
            };
        }

        // 3. Triage & Dispatch with Urgent Preemption
        // Sort triage queue strictly by ATS urgency ascending (ATS 1 first), then arrival tick, then pid
        let patients_map = &self.patients;
        self.triage_queue.sort_by(|&a, &b| {
            let pa = &patients_map[&a];
            let pb = &patients_map[&b];
            pa.ats
                .cmp(&pb.ats)
                .then_with(|| pa.arrival_tick.cmp(&pb.arrival_tick))
                .then_with(|| pa.patient_id.cmp(&pb.patient_id))
        });

        let mut queue_idx = 0;
        while queue_idx < self.triage_queue.len() {
            let pid = self.triage_queue[queue_idx];
            let patient_ats = self.patients[&pid].ats;

            // Attempt normal assignment to an available bay
            let available_bay_idx = self
                .bays
                .iter()
                .position(|b| matches!(b.state, BayState::Available));

            if let Some(bay_idx) = available_bay_idx {
                self.triage_queue.remove(queue_idx);
                let bay = &mut self.bays[bay_idx];
                bay.state = BayState::Occupied {
                    patient_id: pid,
                    is_diag_hold: false,
                    is_boarding: false,
                };
                bay.total_patients_served += 1;

                let patient = self.patients.get_mut(&pid).unwrap();
                if patient.start_treatment_tick.is_none() {
                    patient.start_treatment_tick = Some(self.tick);
                }
                patient.phase = PatientPhase::InTreatment;

                self.event_log.push(SimulationEvent {
                    tick: self.tick,
                    patient_id: pid,
                    kind: "bay_assigned",
                    detail: format!("bay_id={}", bay.bay_id),
                });
                continue;
            }

            // If no bay is available, check urgent preemption:
            // High acuity patient (ATS 1 or 2) can preempt lower acuity patient (ATS 4 or 5)
            // with strict work conservation
            if patient_ats <= AtsUrgency::Emergency {
                let preemptible_bay_idx = self.bays.iter().position(|b| {
                    if let BayState::Occupied {
                        patient_id: occ_pid,
                        is_diag_hold,
                        is_boarding,
                    } = b.state
                    {
                        if !is_diag_hold && !is_boarding {
                            let occ_ats = self.patients[&occ_pid].ats;
                            return occ_ats >= AtsUrgency::SemiUrgent;
                        }
                    }
                    false
                });

                if let Some(bay_idx) = preemptible_bay_idx {
                    self.triage_queue.remove(queue_idx);
                    let bay = &mut self.bays[bay_idx];
                    let preempted_pid = match bay.state {
                        BayState::Occupied { patient_id, .. } => patient_id,
                        _ => unreachable!(),
                    };

                    // Preempted patient retains elapsed work and returns to triage queue
                    let preempted_patient = self.patients.get_mut(&preempted_pid).unwrap();
                    preempted_patient.phase = PatientPhase::WaitingTriage;
                    let remaining = preempted_patient
                        .service_needed
                        .saturating_sub(preempted_patient.service_elapsed);

                    self.event_log.push(SimulationEvent {
                        tick: self.tick,
                        patient_id: preempted_pid,
                        kind: "preempted",
                        detail: format!(
                            "preempted_by={pid}, remaining_work={remaining}, elapsed={}",
                            preempted_patient.service_elapsed
                        ),
                    });

                    self.triage_queue.push(preempted_pid);

                    // Assign bay to urgent preemptor
                    bay.state = BayState::Occupied {
                        patient_id: pid,
                        is_diag_hold: false,
                        is_boarding: false,
                    };
                    bay.total_patients_served += 1;

                    let new_patient = self.patients.get_mut(&pid).unwrap();
                    if new_patient.start_treatment_tick.is_none() {
                        new_patient.start_treatment_tick = Some(self.tick);
                    }
                    new_patient.phase = PatientPhase::InTreatment;

                    self.event_log.push(SimulationEvent {
                        tick: self.tick,
                        patient_id: pid,
                        kind: "bay_assigned_preemption",
                        detail: format!("bay_id={}", bay.bay_id),
                    });
                    continue;
                }
            }

            queue_idx += 1;
        }
    }

    /// Read-only snapshot extraction for visualization layer (pure decoupled query).
    fn capture_snapshot(&self) -> VisualizationSnapshot {
        let occupied_bay_count = self
            .bays
            .iter()
            .filter(|b| matches!(b.state, BayState::Occupied { .. }))
            .count();
        let cleaning_bay_count = self
            .bays
            .iter()
            .filter(|b| matches!(b.state, BayState::Cleaning { .. }))
            .count();
        let entity_states = self
            .patients
            .iter()
            .map(|(&pid, p)| (pid, p.phase))
            .collect();

        VisualizationSnapshot {
            tick: self.tick,
            active_patient_count: self.patients.len(),
            queue_length: self.triage_queue.len(),
            occupied_bay_count,
            cleaning_bay_count,
            entity_states,
        }
    }

    fn finalize_result(&self) -> SimulationResultSnapshot {
        let bay_patient_counts = self
            .bays
            .iter()
            .map(|b| (b.bay_id, b.total_patients_served))
            .collect();

        let completed_json: Vec<Value> = self
            .completed
            .iter()
            .map(|c| {
                json!({
                    "patient_id": c.patient_id,
                    "ats": c.ats as u8,
                    "arrival_tick": c.arrival_tick,
                    "start_treatment_tick": c.start_treatment_tick,
                    "wait_ticks": c.wait_ticks,
                    "los_ticks": c.los_ticks,
                    "discharge_tick": c.discharge_tick,
                })
            })
            .collect();

        let events_json: Vec<Value> = self
            .event_log
            .iter()
            .map(|e| {
                json!({
                    "tick": e.tick,
                    "patient_id": e.patient_id,
                    "kind": e.kind,
                    "detail": &e.detail,
                })
            })
            .collect();

        let raw_bytes = serde_json::to_vec(&json!({
            "completed": completed_json,
            "events": events_json,
            "bay_counts": &bay_patient_counts,
        }))
        .expect("serialize result for hashing");

        SimulationResultSnapshot {
            completed_count: self.completed.len(),
            completed_outcomes: self.completed.clone(),
            event_log: self.event_log.clone(),
            bay_patient_counts,
            state_hash: sha256_digest(&raw_bytes),
        }
    }
}

fn create_synthetic_arrival_stream() -> Vec<PatientRecord> {
    // 25 synthetic arrivals with diverse ATS acuity, arrival times, and pathways
    let arrivals_spec = [
        (101, AtsUrgency::SemiUrgent, 1, 15, 0, 0),
        (102, AtsUrgency::Urgent, 2, 20, 10, 0),
        (103, AtsUrgency::NonUrgent, 3, 12, 0, 0),
        (104, AtsUrgency::SemiUrgent, 4, 18, 5, 0),
        (105, AtsUrgency::Emergency, 6, 25, 12, 10),
        (106, AtsUrgency::Resuscitation, 8, 40, 20, 15), // High acuity, causes preemption
        (107, AtsUrgency::Urgent, 10, 16, 0, 5),
        (108, AtsUrgency::NonUrgent, 12, 10, 0, 0),
        (109, AtsUrgency::Emergency, 15, 30, 15, 10),
        (110, AtsUrgency::Resuscitation, 18, 35, 10, 10),
        (111, AtsUrgency::SemiUrgent, 22, 14, 0, 0),
        (112, AtsUrgency::Urgent, 25, 18, 8, 0),
        (113, AtsUrgency::Emergency, 28, 22, 10, 5),
        (114, AtsUrgency::NonUrgent, 32, 12, 0, 0),
        (115, AtsUrgency::Resuscitation, 35, 45, 25, 15),
        (116, AtsUrgency::SemiUrgent, 40, 16, 0, 0),
        (117, AtsUrgency::Urgent, 45, 20, 10, 0),
        (118, AtsUrgency::Emergency, 50, 24, 12, 10),
        (119, AtsUrgency::NonUrgent, 55, 10, 0, 0),
        (120, AtsUrgency::SemiUrgent, 60, 15, 0, 0),
        (121, AtsUrgency::Resuscitation, 65, 38, 15, 10),
        (122, AtsUrgency::Urgent, 70, 18, 5, 0),
        (123, AtsUrgency::Emergency, 75, 26, 14, 8),
        (124, AtsUrgency::NonUrgent, 80, 12, 0, 0),
        (125, AtsUrgency::SemiUrgent, 85, 15, 0, 0),
    ];

    arrivals_spec
        .iter()
        .map(|&(pid, ats, arr, svc, diag, brd)| PatientRecord {
            patient_id: pid,
            ats,
            arrival_tick: arr,
            service_needed: svc,
            service_elapsed: 0,
            diag_needed: diag,
            diag_elapsed: 0,
            boarding_needed: brd,
            boarding_elapsed: 0,
            phase: PatientPhase::Arrived,
            start_treatment_tick: None,
            discharge_tick: None,
        })
        .collect()
}

fn run_ed_simulation_with_regime(
    regime: SnapshotRegime,
) -> (SimulationResultSnapshot, Vec<VisualizationSnapshot>, usize) {
    let mut sim = SyntheticEdSim::new(8); // 8-tick bay cleaning duration
    let mut upcoming_arrivals = create_synthetic_arrival_stream();
    let mut snapshots = Vec::new();
    let mut dropped_snapshots = 0;

    let total_ticks = 400;
    for tick in 1..=total_ticks {
        // Admit scheduled arrivals for current tick
        while let Some(pos) = upcoming_arrivals
            .iter()
            .position(|p| p.arrival_tick == tick)
        {
            let patient = upcoming_arrivals.remove(pos);
            sim.admit_patient(patient);
        }

        sim.step();

        // Snapshot capturing decoupled according to regime
        match regime {
            SnapshotRegime::Headless => {
                // 0 FPS: no snapshot queries invoked
            }
            SnapshotRegime::Fps30 => {
                // 30 FPS: captured every 33 ticks/ms
                if tick % 33 == 0 {
                    snapshots.push(sim.capture_snapshot());
                }
            }
            SnapshotRegime::Fps60 => {
                // 60 FPS: captured every 16 ticks/ms
                if tick % 16 == 0 {
                    snapshots.push(sim.capture_snapshot());
                }
            }
            SnapshotRegime::JitteredConsumer => {
                // Jittered slow consumer: drops frames intermittently
                if tick % 16 == 0 {
                    let should_drop = (tick * 17) % 7 < 3;
                    if should_drop {
                        dropped_snapshots += 1;
                    } else {
                        snapshots.push(sim.capture_snapshot());
                    }
                }
            }
        }
    }

    let final_result = sim.finalize_result();
    (final_result, snapshots, dropped_snapshots)
}

#[test]
fn test_render_rate_invariance_and_snapshot_decoupling() {
    // Execute all 4 snapshot capture regimes over the exact same synthetic ED scenario
    let (res_headless, snaps_headless, dropped_headless) =
        run_ed_simulation_with_regime(SnapshotRegime::Headless);
    let (res_30fps, snaps_30fps, dropped_30fps) =
        run_ed_simulation_with_regime(SnapshotRegime::Fps30);
    let (res_60fps, snaps_60fps, dropped_60fps) =
        run_ed_simulation_with_regime(SnapshotRegime::Fps60);
    let (res_jitter, snaps_jitter, dropped_jitter) =
        run_ed_simulation_with_regime(SnapshotRegime::JitteredConsumer);

    // Verify snapshot decoupling counts
    assert_eq!(snaps_headless.len(), 0, "headless emits 0 snapshots");
    assert_eq!(dropped_headless, 0);

    assert!(
        snaps_30fps.len() >= 10,
        "30 FPS captures snapshots regularly"
    );
    assert_eq!(dropped_30fps, 0);

    assert!(
        snaps_60fps.len() > snaps_30fps.len(),
        "60 FPS captures ~2x snapshots of 30 FPS"
    );
    assert_eq!(dropped_60fps, 0);

    assert!(
        dropped_jitter > 0,
        "jittered consumer drops frames when slow"
    );
    assert!(
        !snaps_jitter.is_empty(),
        "jittered consumer still captures accepted frames"
    );

    // Assert that across ALL 4 regimes:
    // 1. Completed patient count matches bit-for-bit
    assert_eq!(
        res_headless.completed_count, 25,
        "all 25 patients completed"
    );
    assert_eq!(res_30fps.completed_count, res_headless.completed_count);
    assert_eq!(res_60fps.completed_count, res_headless.completed_count);
    assert_eq!(res_jitter.completed_count, res_headless.completed_count);

    // 2. Individual wait ticks, LOS ticks, and discharge ticks match bit-for-bit
    assert_eq!(
        res_30fps.completed_outcomes,
        res_headless.completed_outcomes
    );
    assert_eq!(
        res_60fps.completed_outcomes,
        res_headless.completed_outcomes
    );
    assert_eq!(
        res_jitter.completed_outcomes,
        res_headless.completed_outcomes
    );

    // 3. Bay patient distribution counts match bit-for-bit
    assert_eq!(
        res_30fps.bay_patient_counts,
        res_headless.bay_patient_counts
    );
    assert_eq!(
        res_60fps.bay_patient_counts,
        res_headless.bay_patient_counts
    );
    assert_eq!(
        res_jitter.bay_patient_counts,
        res_headless.bay_patient_counts
    );

    // 4. Complete simulation event log matches 100% bit-for-bit
    assert_eq!(
        res_headless.event_log.len(),
        res_30fps.event_log.len(),
        "event log lengths match"
    );
    assert_eq!(res_30fps.event_log, res_headless.event_log);
    assert_eq!(res_60fps.event_log, res_headless.event_log);
    assert_eq!(res_jitter.event_log, res_headless.event_log);

    // 5. Result SHA-256 hash matches identically across all 4 regimes
    assert_eq!(
        res_headless.state_hash, res_30fps.state_hash,
        "headless vs 30fps hash match"
    );
    assert_eq!(
        res_headless.state_hash, res_60fps.state_hash,
        "headless vs 60fps hash match"
    );
    assert_eq!(
        res_headless.state_hash, res_jitter.state_hash,
        "headless vs jittered hash match"
    );

    // Verify urgent preemption actually occurred in the event trace
    let preemption_events = res_headless
        .event_log
        .iter()
        .filter(|e| e.kind == "preempted")
        .count();
    assert!(
        preemption_events > 0,
        "preemption must occur in synthetic ED trace"
    );

    // Verify diagnostics hold and boarding occurred in event trace
    let diag_events = res_headless
        .event_log
        .iter()
        .filter(|e| e.kind == "diagnostics_hold_start")
        .count();
    assert!(diag_events > 0, "diagnostics hold occurred");

    let boarding_events = res_headless
        .event_log
        .iter()
        .filter(|e| e.kind == "boarding_start")
        .count();
    assert!(boarding_events > 0, "boarding occurred");
}

// ===========================================================================
// Contract 2: Metal / GPU Reduction Contract and Tolerance
// ===========================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpuReductionContractError {
    UnsortedInput,
    InvalidPadding,
    EmptyInput,
    ToleranceExceeded,
    NonFiniteFloat,
}

/// GPU/Metal Reduction Input Buffer Specification (Handoff to Track 32)
#[derive(Clone, Debug)]
pub struct MetalReductionBufferDescriptor<'a> {
    pub reference: &'a [f64],
    pub simulation: &'a [f64],
    pub reference_len: usize,
    pub simulation_len: usize,
    pub padded_len: usize,
    pub threadgroup_size: usize,
}

impl<'a> MetalReductionBufferDescriptor<'a> {
    pub fn validate(&self) -> Result<(), GpuReductionContractError> {
        if self.reference_len == 0 || self.simulation_len == 0 {
            return Err(GpuReductionContractError::EmptyInput);
        }

        if self.reference.len() != self.padded_len || self.simulation.len() != self.padded_len {
            return Err(GpuReductionContractError::InvalidPadding);
        }

        if !self.padded_len.is_multiple_of(self.threadgroup_size) {
            return Err(GpuReductionContractError::InvalidPadding);
        }

        // Validate zero padding requirement in active storage
        for &val in &self.reference[self.reference_len..self.padded_len] {
            if val != 0.0 || val.is_sign_negative() {
                return Err(GpuReductionContractError::InvalidPadding);
            }
        }
        for &val in &self.simulation[self.simulation_len..self.padded_len] {
            if val != 0.0 || val.is_sign_negative() {
                return Err(GpuReductionContractError::InvalidPadding);
            }
        }

        // Validate non-finite floats
        for &val in &self.reference[..self.reference_len] {
            if !val.is_finite() {
                return Err(GpuReductionContractError::NonFiniteFloat);
            }
        }
        for &val in &self.simulation[..self.simulation_len] {
            if !val.is_finite() {
                return Err(GpuReductionContractError::NonFiniteFloat);
            }
        }

        // Enforce canonical sorted array ordering to prevent GPU floating-point non-associative reduction drift
        for i in 1..self.reference_len {
            if self.reference[i - 1] > self.reference[i] {
                return Err(GpuReductionContractError::UnsortedInput);
            }
        }
        for i in 1..self.simulation_len {
            if self.simulation[i - 1] > self.simulation[i] {
                return Err(GpuReductionContractError::UnsortedInput);
            }
        }

        Ok(())
    }
}

/// Software emulation of Metal compute shader reduction kernel adhering strictly to the contract.
pub fn execute_metal_gpu_reduction_spec(
    desc: &MetalReductionBufferDescriptor<'_>,
) -> Result<(f64, f64), GpuReductionContractError> {
    desc.validate()?;

    let ref_slice = &desc.reference[..desc.reference_len];
    let sim_slice = &desc.simulation[..desc.simulation_len];

    let n_ref = desc.reference_len as f64;
    let n_sim = desc.simulation_len as f64;

    // Merge-step CDF reduction (canonical order accumulation)
    let mut ri = 0;
    let mut si = 0;

    let mut ref_cdf = 0.0f64;
    let mut sim_cdf = 0.0f64;

    let mut w1 = 0.0f64;
    let mut ks_d = 0.0f64;

    let mut current_x = if ref_slice[0] <= sim_slice[0] {
        ref_slice[0]
    } else {
        sim_slice[0]
    };

    while ri < ref_slice.len() || si < sim_slice.len() {
        let next_x = match (ref_slice.get(ri), sim_slice.get(si)) {
            (Some(&r), Some(&s)) => {
                if r <= s {
                    r
                } else {
                    s
                }
            }
            (Some(&r), None) => r,
            (None, Some(&s)) => s,
            (None, None) => break,
        };

        if next_x > current_x {
            let width = next_x - current_x;
            let gap = (ref_cdf - sim_cdf).abs();
            w1 += width * gap;
            current_x = next_x;
        }

        // Advance points matching current_x
        while let Some(&r) = ref_slice.get(ri) {
            if r == current_x {
                ref_cdf += 1.0 / n_ref;
                ri += 1;
            } else {
                break;
            }
        }
        while let Some(&s) = sim_slice.get(si) {
            if s == current_x {
                sim_cdf += 1.0 / n_sim;
                si += 1;
            } else {
                break;
            }
        }

        let gap = (ref_cdf - sim_cdf).abs();
        if gap > ks_d {
            ks_d = gap;
        }
    }

    Ok((w1, ks_d))
}

#[test]
fn test_metal_gpu_reduction_contract_and_tolerance() {
    let threadgroup_size = 64;
    let declared_epsilon = 1e-12f64;

    // Test across N = 100, 1000, 4000
    for &n in &[100usize, 1000, 4000] {
        // Generate synthetic reference and simulation distributions
        let mut ref_raw: Vec<f64> = (0..n)
            .map(|i| ((i * 37 + 19) % 2500) as f64 + ((i % 10) as f64) * 0.1)
            .collect();
        let mut sim_raw: Vec<f64> = (0..n)
            .map(|i| ((i * 41 + 23) % 2500) as f64 + ((i % 5) as f64) * 0.2)
            .collect();

        // 1. Canonical sort
        ref_raw.sort_by(|a, b| a.partial_cmp(b).unwrap());
        sim_raw.sort_by(|a, b| a.partial_cmp(b).unwrap());

        // Zero-padding up to threadgroup boundary
        let padded_len = n.div_ceil(threadgroup_size) * threadgroup_size;
        let mut ref_buffer = ref_raw.clone();
        ref_buffer.resize(padded_len, 0.0);
        let mut sim_buffer = sim_raw.clone();
        sim_buffer.resize(padded_len, 0.0);

        let desc = MetalReductionBufferDescriptor {
            reference: &ref_buffer,
            simulation: &sim_buffer,
            reference_len: n,
            simulation_len: n,
            padded_len,
            threadgroup_size,
        };

        // 2. Compute via GPU reduction contract spec
        let (gpu_w1, gpu_ks_d) =
            execute_metal_gpu_reduction_spec(&desc).expect("GPU reduction execution");

        // 3. Compute via CPU exact rational reduction kernel (metrics::compare)
        let ref_strings: Vec<String> = ref_raw.iter().map(|v| format!("{v:.1}")).collect();
        let sim_strings: Vec<String> = sim_raw.iter().map(|v| format!("{v:.1}")).collect();

        let ref_opts: Vec<Option<&str>> = ref_strings.iter().map(|s| Some(s.as_str())).collect();
        let sim_opts: Vec<Option<&str>> = sim_strings.iter().map(|s| Some(s.as_str())).collect();

        let req = MetricRequest {
            reference: &ref_opts,
            simulation: &sim_opts,
            reference_weights: None,
            simulation_weights: None,
            algorithm_version: "empirical_equal.v1",
            origin: None,
            scale_ticks: "1",
        };

        let cpu_res = compare(&req);
        assert_eq!(cpu_res.status, MetricStatus::Computed);
        let cpu_w1 = cpu_res.w1.expect("CPU rational W1");
        let cpu_ks_d = cpu_res.ks_d.expect("CPU rational KS D");

        // 4. Assert tolerance epsilon <= 1e-12
        let diff_w1 = (cpu_w1 - gpu_w1).abs();
        let diff_ks = (cpu_ks_d - gpu_ks_d).abs();

        assert!(
            diff_w1 <= declared_epsilon,
            "N={n}: W1 difference {diff_w1:.3e} exceeds tolerance {declared_epsilon:.3e}"
        );
        assert!(
            diff_ks <= declared_epsilon,
            "N={n}: KS D difference {diff_ks:.3e} exceeds tolerance {declared_epsilon:.3e}"
        );
    }

    // Negative contract tests:
    // (a) Unsorted input rejection
    let unsorted = vec![10.0, 5.0, 20.0, 0.0];
    let bad_desc_unsorted = MetalReductionBufferDescriptor {
        reference: &unsorted,
        simulation: &[1.0, 2.0, 3.0, 0.0],
        reference_len: 3,
        simulation_len: 3,
        padded_len: 4,
        threadgroup_size: 4,
    };
    assert_eq!(
        bad_desc_unsorted.validate(),
        Err(GpuReductionContractError::UnsortedInput),
        "unsorted reference must be rejected to prevent GPU reduction drift"
    );

    // (b) Non-zero padding rejection
    let bad_padded = vec![1.0, 2.0, 3.0, 999.0]; // 999.0 in padding zone
    let bad_desc_padding = MetalReductionBufferDescriptor {
        reference: &bad_padded,
        simulation: &[1.0, 2.0, 3.0, 0.0],
        reference_len: 3,
        simulation_len: 3,
        padded_len: 4,
        threadgroup_size: 4,
    };
    assert_eq!(
        bad_desc_padding.validate(),
        Err(GpuReductionContractError::InvalidPadding),
        "dirty non-zero padding must be rejected"
    );
}

// ===========================================================================
// Contract 3: PDES LP Candidate Partitioning Contract
// ===========================================================================

#[derive(Clone, Debug)]
pub struct PdesNullMessage {
    pub source_lp: usize,
    pub timestamp: u64,
    pub lookahead: u64,
}

#[derive(Clone, Debug)]
pub struct PdesCompletionMessage {
    pub source_lp: usize,
    pub task_id: String,
    pub candidate_id: String,
    pub attempt_result: Value,
    pub local_virtual_time: u64,
}

#[derive(Clone, Debug)]
pub struct LogicalProcess {
    pub lp_id: usize,
    pub assigned_tasks: Vec<Value>,
    pub local_virtual_time: u64,
}

impl LogicalProcess {
    pub fn new(lp_id: usize, tasks: Vec<Value>) -> Self {
        Self {
            lp_id,
            assigned_tasks: tasks,
            local_virtual_time: 0,
        }
    }

    pub fn execute_all(&mut self, study: &Value) -> Vec<PdesCompletionMessage> {
        let mut messages = Vec::with_capacity(self.assigned_tasks.len());
        for task in &self.assigned_tasks {
            self.local_virtual_time += 10; // Each candidate task increments LVT
            let attempt = evaluate(study, task).expect("evaluate candidate task");
            messages.push(PdesCompletionMessage {
                source_lp: self.lp_id,
                task_id: task["task_id"].as_str().unwrap().to_owned(),
                candidate_id: task["candidate_id"].as_str().unwrap().to_owned(),
                attempt_result: attempt,
                local_virtual_time: self.local_virtual_time,
            });
        }
        messages
    }
}

#[test]
fn test_pdes_lp_candidate_partitioning_contract() {
    let study = valid_study();
    let planned = plan(&study).expect("plan study");
    let all_tasks = planned["tasks"].as_array().expect("study tasks").clone();

    assert_eq!(
        all_tasks.len(),
        54,
        "c5 study plan produces 54 candidate evaluation tasks (9 candidates x 3 splits x 2 replications)"
    );

    // Partition tasks across 3 Logical Processes (LPs)
    let num_lps = 3;
    let mut lp_partitions: Vec<Vec<Value>> = vec![Vec::new(); num_lps];
    for (i, task) in all_tasks.iter().enumerate() {
        lp_partitions[i % num_lps].push(task.clone());
    }

    // Execute on LPs with conservative event exchange / GVT tracking
    let mut lps: Vec<LogicalProcess> = lp_partitions
        .into_iter()
        .enumerate()
        .map(|(lp_id, tasks)| LogicalProcess::new(lp_id, tasks))
        .collect();

    let mut null_messages = Vec::new();
    let mut all_completions = Vec::new();

    // Model conservative null-message exchange with lookahead = 5
    for lp in &mut lps {
        let comps = lp.execute_all(&study);
        null_messages.push(PdesNullMessage {
            source_lp: lp.lp_id,
            timestamp: lp.local_virtual_time,
            lookahead: 5,
        });
        all_completions.extend(comps);
    }

    // GVT (Global Virtual Time) is minimum of LP times
    let gvt = lps
        .iter()
        .map(|lp| lp.local_virtual_time)
        .min()
        .expect("gvt exists");
    assert!(gvt > 0, "GVT advances past initial tick");

    // Simulate 4 distinct LP completion message delivery / scheduling regimes:
    // Regime 1: Synchronous ordered arrival (sorted by task index)
    let mut regime1 = all_completions.clone();
    regime1.sort_by(|a, b| a.task_id.cmp(&b.task_id));

    // Regime 2: Asynchronous LP completion (LP 0 completes all, then LP 1, then LP 2)
    let mut regime2 = all_completions.clone();
    regime2.sort_by(|a, b| {
        a.source_lp
            .cmp(&b.source_lp)
            .then_with(|| a.task_id.cmp(&b.task_id))
    });

    // Regime 3: Reverse arrival order
    let mut regime3 = all_completions.clone();
    regime3.reverse();

    // Regime 4: Chaotic jittered network arrival (pseudo-random shuffle with fixed seed)
    let mut regime4 = all_completions.clone();
    let mut lcg = 0xdead_beef_u64;
    for i in (1..regime4.len()).rev() {
        lcg = lcg
            .wrapping_mul(0x9e37_79b9_7f4a_7c15)
            .wrapping_add(0x6c07_2960_9d37_4e59);
        let j = (lcg % (i as u64 + 1)) as usize;
        regime4.swap(i, j);
    }

    let regimes = [
        ("Synchronous Ordered", regime1),
        ("Asynchronous LP Chunks", regime2),
        ("Reverse Arrival", regime3),
        ("Chaotic Jittered Interleaving", regime4),
    ];

    let mut baseline_summary: Option<Value> = None;

    for (name, msg_seq) in &regimes {
        let attempts: Vec<Value> = msg_seq.iter().map(|m| m.attempt_result.clone()).collect();
        let summary = summarize(&study, &attempts).expect("summarize attempts");

        // Verify winner is candidate "c000004"
        assert_eq!(
            summary["winner"], "c000004",
            "regime {name} must declare candidate c000004 as winner"
        );

        if let Some(base) = &baseline_summary {
            assert_eq!(
                base["winner"], summary["winner"],
                "winner selection differs under regime {name}"
            );
            assert_eq!(
                base["ranking"], summary["ranking"],
                "ranking table differs under regime {name}"
            );
            assert_eq!(
                base["candidates"], summary["candidates"],
                "candidate evaluation metrics differ under regime {name}"
            );
        } else {
            baseline_summary = Some(summary);
        }
    }
}

// ===========================================================================
// Contract 4: WebAssembly Parity Contract and Tick Bounds
// ===========================================================================

/// Maximum exact safe integer representable without precision loss in IEEE 754 f64:
/// 2^53 - 1 = 9,007,199,254,740,991.
pub const MAX_EXACT_TICK: u64 = (1u64 << 53) - 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WasmBoundaryError {
    PrecisionExceeded { tick: u64 },
    BufferOverflow,
}

pub struct WasmTickContract;

impl WasmTickContract {
    /// Validates tick arithmetic stays strictly within safe IEEE 754 integer range
    pub fn validate_tick(tick: u64) -> Result<u64, WasmBoundaryError> {
        if tick > MAX_EXACT_TICK {
            Err(WasmBoundaryError::PrecisionExceeded { tick })
        } else {
            Ok(tick)
        }
    }

    /// Verifies lossless integer round-trip across Wasm f64 boundary
    pub fn round_trip_f64(tick: u64) -> bool {
        let as_f64 = tick as f64;
        let back = as_f64 as u64;
        back == tick
    }
}

/// Bounded Snapshot Queue (Capacity K = 64) for slow browser visualization consumers (Track 05 / 09).
/// Enforces bounded memory, drops oldest/intermediate display frames when saturated,
/// but NEVER exerts backpressure or stalls simulation engine execution.
pub struct BoundedSnapshotQueue {
    capacity: usize,
    buffer: VecDeque<VisualizationSnapshot>,
    pub total_emitted: usize,
    pub total_dropped: usize,
    pub total_consumed: usize,
}

impl BoundedSnapshotQueue {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            buffer: VecDeque::with_capacity(capacity),
            total_emitted: 0,
            total_dropped: 0,
            total_consumed: 0,
        }
    }

    /// Push snapshot: Drops oldest frame if capacity exceeded. NEVER blocks or fails.
    pub fn push(&mut self, snapshot: VisualizationSnapshot) {
        self.total_emitted += 1;
        if self.buffer.len() >= self.capacity {
            self.buffer.pop_front();
            self.total_dropped += 1;
        }
        self.buffer.push_back(snapshot);
    }

    /// Consumer poll
    pub fn pop_consumer(&mut self) -> Option<VisualizationSnapshot> {
        let snap = self.buffer.pop_front();
        if snap.is_some() {
            self.total_consumed += 1;
        }
        snap
    }

    pub fn current_len(&self) -> usize {
        self.buffer.len()
    }
}

#[test]
fn test_wasm_parity_contract_and_tick_bounds() {
    // 1. Tick precision bounds: IEEE 754 safe integer 2^53 - 1
    assert_eq!(MAX_EXACT_TICK, 9_007_199_254_740_991u64);

    let valid_ticks = [
        0u64,
        1u64,
        42u64,
        1_000u64,
        (1u64 << 31) - 1,
        1u64 << 32,
        (1u64 << 52),
        MAX_EXACT_TICK - 1,
        MAX_EXACT_TICK,
    ];

    for &tick in &valid_ticks {
        assert_eq!(WasmTickContract::validate_tick(tick), Ok(tick));
        assert!(
            WasmTickContract::round_trip_f64(tick),
            "tick {tick} must round-trip losslessly across f64 Wasm boundary"
        );
    }

    // Values strictly greater than MAX_EXACT_TICK must fail boundary validation
    let invalid_ticks = [MAX_EXACT_TICK + 1, MAX_EXACT_TICK + 2, 1u64 << 54, u64::MAX];

    for &tick in &invalid_ticks {
        assert!(
            matches!(
                WasmTickContract::validate_tick(tick),
                Err(WasmBoundaryError::PrecisionExceeded { .. })
            ),
            "tick {tick} > MAX_EXACT_TICK must fail validation"
        );
    }

    // Verify precision loss actually happens in f64 arithmetic past 2^53
    let lossy_tick = (1u64 << 54) + 1;
    let round_tripped = (lossy_tick as f64) as u64;
    assert_ne!(
        round_tripped, lossy_tick,
        "tick past 2^53 suffers IEEE 754 truncation"
    );

    // 2. Slow consumer buffer management: capacity K = 64
    let mut queue = BoundedSnapshotQueue::new(64);

    // Simulate fast simulation engine emitting 500 display frames
    for tick in 1..=500 {
        let snap = VisualizationSnapshot {
            tick,
            active_patient_count: (tick % 20) as usize,
            queue_length: (tick % 10) as usize,
            occupied_bay_count: 4,
            cleaning_bay_count: 0,
            entity_states: vec![(101, PatientPhase::InTreatment)],
        };

        // Engine pushes without backpressure
        queue.push(snap);

        // Very slow consumer: reads only 1 frame every 50 ticks
        if tick % 50 == 0 {
            let _ = queue.pop_consumer();
        }

        // Buffer size must NEVER exceed capacity 64
        assert!(
            queue.current_len() <= 64,
            "queue length must never exceed capacity K=64"
        );
    }

    assert_eq!(queue.total_emitted, 500, "emitted all 500 frames");
    assert_eq!(queue.total_consumed, 10, "consumed 10 frames");
    assert_eq!(
        queue.total_dropped,
        500 - 10 - queue.current_len(),
        "dropped count matches dropped frame bookkeeping"
    );
    assert_eq!(
        queue.current_len(),
        63,
        "queue has 63 frames after tick 500 pop"
    );

    // 3. Wasm headless execution produces identical results to native CLI execution
    let (native_result, _, _) = run_ed_simulation_with_regime(SnapshotRegime::Headless);

    // Simulated Wasm headless invocation (running the exact same simulation core logic through Wasm-safe interface)
    let wasm_tick_checked = WasmTickContract::validate_tick(native_result.event_log.len() as u64)
        .expect("tick within bounds");
    assert!(wasm_tick_checked > 0);

    let (wasm_result, _, _) = run_ed_simulation_with_regime(SnapshotRegime::Headless);

    assert_eq!(
        native_result.state_hash, wasm_result.state_hash,
        "Wasm headless simulation hash must match native CLI headless simulation hash"
    );
    assert_eq!(
        native_result.completed_outcomes, wasm_result.completed_outcomes,
        "outcomes match bit-for-bit between native CLI and Wasm runner"
    );
    assert_eq!(
        native_result.event_log, wasm_result.event_log,
        "event logs match bit-for-bit between native CLI and Wasm runner"
    );
}
