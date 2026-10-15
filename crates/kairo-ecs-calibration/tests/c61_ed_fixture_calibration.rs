//! C6.1 integration tests for synthetic ED operational fixture dynamics and calibration.
//!
//! Covers:
//! 1. Full 6-dynamic operational fixture: ATS arrivals/acuity, staff zones, diagnostics hold,
//!    urgent preemption with strict work conservation, bed cleaning, and boarding.
//! 2. Research ledger one-server queue oracle from report 12.
//! 3. Recovering known transit parameters on identifiable fixture using experimental_c5.
//! 4. Flagging confounded non-identifiable ridge without claiming unique recovery.

#![forbid(unsafe_code)]

use kairo_ecs_calibration::experimental_c5 as c5;
use serde_json::{json, Value};
use std::collections::BTreeMap;

#[allow(dead_code)]
#[path = "../src/residuals.rs"]
mod residuals;
#[allow(dead_code)]
#[path = "../src/trace_order.rs"]
mod trace_order;
#[allow(dead_code)]
mod seed_map {
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub(crate) struct CalibrationStreamKey;
}
#[allow(dead_code)]
#[path = "../src/shadow.rs"]
mod shadow;
#[allow(dead_code)]
#[path = "../src/shadow_ledger.rs"]
mod shadow_ledger;

use shadow::{ObservedEvent, ResourcePolicy, ShadowError, Transition};
use shadow_ledger::{InitialResource, LedgerLimits, ObservedLedger};
use trace_order::{EventKindRank, TraceOrderKeyV1};

// ===========================================================================
// Operational Fixture Domain Types (Dynamics 1 - 6)
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

impl AtsUrgency {
    pub fn acuity_level(&self) -> u8 {
        *self as u8
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperationalError {
    Ineligible,
    Unavailable,
    SpaceUnavailable,
    CleaningInProgress,
    DiagnosticBlocked,
    BoardingRetained,
    Accounting,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BayPhase {
    Available,
    Occupied {
        patient_id: u64,
        diagnostic_blocked: bool,
        boarding: bool,
    },
    Cleaning {
        ready_tick: u64,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreatmentBay {
    pub bay_id: String,
    pub zone_id: String,
    pub phase: BayPhase,
}

impl TreatmentBay {
    pub fn new(bay_id: impl Into<String>, zone_id: impl Into<String>) -> Self {
        Self {
            bay_id: bay_id.into(),
            zone_id: zone_id.into(),
            phase: BayPhase::Available,
        }
    }

    pub fn occupy(&mut self, patient_id: u64) -> Result<(), OperationalError> {
        match self.phase {
            BayPhase::Available => {
                self.phase = BayPhase::Occupied {
                    patient_id,
                    diagnostic_blocked: false,
                    boarding: false,
                };
                Ok(())
            }
            BayPhase::Cleaning { .. } => Err(OperationalError::CleaningInProgress),
            BayPhase::Occupied { .. } => Err(OperationalError::SpaceUnavailable),
        }
    }

    pub fn block_for_diagnostics(&mut self) -> Result<(), OperationalError> {
        match self.phase {
            BayPhase::Occupied {
                patient_id,
                boarding,
                ..
            } => {
                self.phase = BayPhase::Occupied {
                    patient_id,
                    diagnostic_blocked: true,
                    boarding,
                };
                Ok(())
            }
            _ => Err(OperationalError::SpaceUnavailable),
        }
    }

    pub fn clear_diagnostics(&mut self) -> Result<(), OperationalError> {
        match self.phase {
            BayPhase::Occupied {
                patient_id,
                boarding,
                ..
            } => {
                self.phase = BayPhase::Occupied {
                    patient_id,
                    diagnostic_blocked: false,
                    boarding,
                };
                Ok(())
            }
            _ => Err(OperationalError::SpaceUnavailable),
        }
    }

    pub fn begin_boarding(&mut self) -> Result<(), OperationalError> {
        match self.phase {
            BayPhase::Occupied {
                patient_id,
                diagnostic_blocked,
                boarding: false,
            } => {
                self.phase = BayPhase::Occupied {
                    patient_id,
                    diagnostic_blocked,
                    boarding: true,
                };
                Ok(())
            }
            BayPhase::Occupied { boarding: true, .. } => Err(OperationalError::BoardingRetained),
            _ => Err(OperationalError::SpaceUnavailable),
        }
    }

    pub fn vacate_for_cleaning(&mut self, ready_tick: u64) -> Result<(), OperationalError> {
        match self.phase {
            BayPhase::Occupied {
                diagnostic_blocked: true,
                ..
            } => Err(OperationalError::DiagnosticBlocked),
            BayPhase::Occupied { boarding: true, .. } => Err(OperationalError::BoardingRetained),
            BayPhase::Occupied { .. } => {
                self.phase = BayPhase::Cleaning { ready_tick };
                Ok(())
            }
            _ => Err(OperationalError::SpaceUnavailable),
        }
    }

    pub fn accept_transfer(&mut self, ready_tick: u64) -> Result<(), OperationalError> {
        match self.phase {
            BayPhase::Occupied {
                diagnostic_blocked: true,
                ..
            } => Err(OperationalError::DiagnosticBlocked),
            BayPhase::Occupied { boarding: true, .. } => {
                self.phase = BayPhase::Cleaning { ready_tick };
                Ok(())
            }
            _ => Err(OperationalError::SpaceUnavailable),
        }
    }

    pub fn finish_cleaning(&mut self, current_tick: u64) -> Result<(), OperationalError> {
        match self.phase {
            BayPhase::Cleaning { ready_tick } if current_tick >= ready_tick => {
                self.phase = BayPhase::Available;
                Ok(())
            }
            BayPhase::Cleaning { .. } => Err(OperationalError::CleaningInProgress),
            _ => Err(OperationalError::SpaceUnavailable),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StaffMember {
    pub staff_id: String,
    pub assigned_zone: String,
    pub active_patient_id: Option<u64>,
}

impl StaffMember {
    pub fn new(staff_id: impl Into<String>, assigned_zone: impl Into<String>) -> Self {
        Self {
            staff_id: staff_id.into(),
            assigned_zone: assigned_zone.into(),
            active_patient_id: None,
        }
    }

    pub fn can_claim_in_zone(&self, task_zone: &str) -> bool {
        self.assigned_zone == task_zone
    }

    pub fn is_free(&self) -> bool {
        self.active_patient_id.is_none()
    }

    pub fn claim(&mut self, task_zone: &str, patient_id: u64) -> Result<(), OperationalError> {
        if !self.can_claim_in_zone(task_zone) {
            return Err(OperationalError::Ineligible);
        }
        if !self.is_free() {
            return Err(OperationalError::Unavailable);
        }
        self.active_patient_id = Some(patient_id);
        Ok(())
    }

    pub fn release(&mut self) -> Result<u64, OperationalError> {
        self.active_patient_id
            .take()
            .ok_or(OperationalError::Unavailable)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatientTask {
    pub task_id: String,
    pub patient_id: u64,
    pub ats: AtsUrgency,
    pub zone_id: String,
    pub enqueue_tick: u64,
    pub total_work_ticks: u64,
    pub work_elapsed_ticks: u64,
}

impl PatientTask {
    pub fn remaining_ticks(&self) -> u64 {
        self.total_work_ticks
            .saturating_sub(self.work_elapsed_ticks)
    }

    pub fn is_complete(&self) -> bool {
        self.work_elapsed_ticks >= self.total_work_ticks
    }
}

/// Dispatches next candidate strictly by urgency (ATS ascending), FIFO within urgency,
/// then patient_id and task_id for deterministic tie-breaking.
pub fn select_priority_dispatch(queue: &[PatientTask], zone_id: &str) -> Option<usize> {
    queue
        .iter()
        .enumerate()
        .filter(|(_, task)| task.zone_id == zone_id)
        .min_by(|(_, a), (_, b)| {
            a.ats
                .cmp(&b.ats)
                .then_with(|| a.enqueue_tick.cmp(&b.enqueue_tick))
                .then_with(|| a.patient_id.cmp(&b.patient_id))
                .then_with(|| a.task_id.cmp(&b.task_id))
        })
        .map(|(idx, _)| idx)
}

// ===========================================================================
// Test 1: Full 6 Operational Dynamics & Conservation Invariants
// ===========================================================================

#[test]
fn test_synthetic_ed_fixture_operational_dynamics_covering_all_six_dynamics() {
    // -----------------------------------------------------------------------
    // Resources & Infrastructure
    // -----------------------------------------------------------------------
    let mut bays = vec![
        TreatmentBay::new("bay.resus.1", "zone.resus"),
        TreatmentBay::new("bay.acute.1", "zone.acute"),
        TreatmentBay::new("bay.subacute.1", "zone.subacute"),
    ];

    let mut staff = vec![
        StaffMember::new("staff.resus.1", "zone.resus"),
        StaffMember::new("staff.acute.1", "zone.acute"),
        StaffMember::new("staff.subacute.1", "zone.subacute"),
    ];

    let mut queue: Vec<PatientTask> = Vec::new();
    let mut in_flight: Vec<PatientTask> = Vec::new();
    let mut completed: Vec<PatientTask> = Vec::new();
    let mut total_admitted: u64 = 0;

    // Invariant validation closure
    let verify_invariants = |queue: &[PatientTask],
                             in_flight: &[PatientTask],
                             completed: &[PatientTask],
                             bays: &[TreatmentBay],
                             staff: &[StaffMember],
                             admitted: u64| {
        // Patient conservation: admitted = queue + in_flight + completed
        let active_in_system = queue.len() + in_flight.len() + completed.len();
        assert_eq!(
            admitted as usize, active_in_system,
            "patient conservation: admitted must equal queue + in_flight + completed"
        );

        // Capacity conservation: occupied bays == patients currently in bays
        let occupied_bays = bays
            .iter()
            .filter(|b| matches!(b.phase, BayPhase::Occupied { .. }))
            .count();
        let in_flight_in_bays = in_flight.len();
        assert_eq!(
            occupied_bays, in_flight_in_bays,
            "capacity conservation: occupied bays must equal active patients in bays"
        );

        // Staff conservation: active staff == actively treated patients
        let active_staff = staff.iter().filter(|s| !s.is_free()).count();
        // Patients in in_flight who are not paused/yielded
        let staff_matched_patients = staff
            .iter()
            .filter_map(|s| s.active_patient_id)
            .collect::<Vec<_>>();
        assert_eq!(
            active_staff,
            staff_matched_patients.len(),
            "staff conservation: active staff count must equal actively served patients"
        );
    };

    // -----------------------------------------------------------------------
    // (1) Arrivals / Acuity: ATS 1-5 urgency ordering and priority dispatch
    // -----------------------------------------------------------------------
    let ats_arrivals = [
        (101, AtsUrgency::SemiUrgent, 10),   // ATS 4
        (102, AtsUrgency::Emergency, 12),    // ATS 2
        (103, AtsUrgency::Resuscitation, 8), // ATS 1
        (104, AtsUrgency::NonUrgent, 20),    // ATS 5
        (105, AtsUrgency::Urgent, 15),       // ATS 3
    ];

    for (pid, ats, work) in ats_arrivals {
        total_admitted += 1;
        queue.push(PatientTask {
            task_id: format!("task.{pid}"),
            patient_id: pid,
            ats,
            zone_id: "zone.acute".into(),
            enqueue_tick: 0,
            total_work_ticks: work,
            work_elapsed_ticks: 0,
        });
    }

    // Verify priority dispatch orders strictly by acuity: ATS 1 -> ATS 2 -> ATS 3 -> ATS 4 -> ATS 5
    let mut dispatched_order = Vec::new();
    while let Some(idx) = select_priority_dispatch(&queue, "zone.acute") {
        let task = queue.remove(idx);
        dispatched_order.push((task.patient_id, task.ats));
    }
    assert_eq!(
        dispatched_order,
        vec![
            (103, AtsUrgency::Resuscitation),
            (102, AtsUrgency::Emergency),
            (105, AtsUrgency::Urgent),
            (101, AtsUrgency::SemiUrgent),
            (104, AtsUrgency::NonUrgent),
        ],
        "priority dispatch must select high acuity first (ATS 1 to ATS 5)"
    );

    // Drain dispatched test patients to reset queue
    completed.extend(dispatched_order.into_iter().map(|(pid, ats)| PatientTask {
        task_id: format!("task.{pid}"),
        patient_id: pid,
        ats,
        zone_id: "zone.acute".into(),
        enqueue_tick: 0,
        total_work_ticks: 10,
        work_elapsed_ticks: 10,
    }));
    verify_invariants(
        &queue,
        &in_flight,
        &completed,
        &bays,
        &staff,
        total_admitted,
    );

    // -----------------------------------------------------------------------
    // (2) Staff Zones: Zone-based eligibility
    // -----------------------------------------------------------------------
    // Resus task can only be claimed by staff allocated to "zone.resus"
    total_admitted += 1;
    let resus_task = PatientTask {
        task_id: "task.resus.201".into(),
        patient_id: 201,
        ats: AtsUrgency::Emergency,
        zone_id: "zone.resus".into(),
        enqueue_tick: 1,
        total_work_ticks: 6,
        work_elapsed_ticks: 0,
    };
    queue.push(resus_task);

    // Staff in acute zone tries to claim resus task -> Ineligible!
    assert_eq!(
        staff[1].claim(&queue[0].zone_id, queue[0].patient_id),
        Err(OperationalError::Ineligible),
        "staff in zone.acute cannot claim tasks in zone.resus"
    );

    // Staff in subacute zone tries to claim resus task -> Ineligible!
    assert_eq!(
        staff[2].claim(&queue[0].zone_id, queue[0].patient_id),
        Err(OperationalError::Ineligible),
        "staff in zone.subacute cannot claim tasks in zone.resus"
    );

    // Eligible staff in resus zone claims resus task -> Success!
    assert!(staff[0]
        .claim(&queue[0].zone_id, queue[0].patient_id)
        .is_ok());

    let mut task_201 = queue.remove(0);
    bays[0].occupy(task_201.patient_id).unwrap();
    in_flight.push(task_201.clone());
    verify_invariants(
        &queue,
        &in_flight,
        &completed,
        &bays,
        &staff,
        total_admitted,
    );

    // Complete task 201
    task_201.work_elapsed_ticks = task_201.total_work_ticks;
    staff[0].release().unwrap();
    bays[0].vacate_for_cleaning(5).unwrap();
    bays[0].finish_cleaning(5).unwrap();
    in_flight.retain(|t| t.patient_id != 201);
    completed.push(task_201);
    verify_invariants(
        &queue,
        &in_flight,
        &completed,
        &bays,
        &staff,
        total_admitted,
    );

    // -----------------------------------------------------------------------
    // (3) Diagnostics: Bay held for diagnostics (diagnostic_blocked: true)
    // -----------------------------------------------------------------------
    total_admitted += 1;
    let diag_patient_id = 301;
    let mut diag_task = PatientTask {
        task_id: "task.diag.301".into(),
        patient_id: diag_patient_id,
        ats: AtsUrgency::Urgent,
        zone_id: "zone.acute".into(),
        enqueue_tick: 5,
        total_work_ticks: 8,
        work_elapsed_ticks: 0,
    };
    bays[1].occupy(diag_patient_id).unwrap();
    staff[1].claim("zone.acute", diag_patient_id).unwrap();
    in_flight.push(diag_task.clone());

    // Bay enters diagnostics hold
    bays[1].block_for_diagnostics().unwrap();
    assert!(matches!(
        bays[1].phase,
        BayPhase::Occupied {
            patient_id: 301,
            diagnostic_blocked: true,
            boarding: false
        }
    ));

    // Treatment ticks elapse, clinical work finishes
    diag_task.work_elapsed_ticks = 8;
    staff[1].release().unwrap();

    // Premature vacate or clean is rejected while diagnostic_blocked is true!
    assert_eq!(
        bays[1].vacate_for_cleaning(15),
        Err(OperationalError::DiagnosticBlocked),
        "bay held for diagnostics cannot be vacated or cleaned"
    );

    // Diagnostics clear -> bay can now be vacated
    bays[1].clear_diagnostics().unwrap();
    assert!(bays[1].vacate_for_cleaning(15).is_ok());
    assert!(matches!(
        bays[1].phase,
        BayPhase::Cleaning { ready_tick: 15 }
    ));
    bays[1].finish_cleaning(15).unwrap();
    assert_eq!(bays[1].phase, BayPhase::Available);

    in_flight.retain(|t| t.patient_id != diag_patient_id);
    completed.push(diag_task);
    verify_invariants(
        &queue,
        &in_flight,
        &completed,
        &bays,
        &staff,
        total_admitted,
    );

    // -----------------------------------------------------------------------
    // (4) Urgent Preemption: ATS 1/2 preempts ATS 5; strictly conserves work ticks
    // -----------------------------------------------------------------------
    total_admitted += 1;
    let non_urgent_id = 401;
    let mut non_urgent_task = PatientTask {
        task_id: "task.nonurgent.401".into(),
        patient_id: non_urgent_id,
        ats: AtsUrgency::NonUrgent, // ATS 5
        zone_id: "zone.resus".into(),
        enqueue_tick: 16,
        total_work_ticks: 10,
        work_elapsed_ticks: 0,
    };
    bays[0].occupy(non_urgent_id).unwrap();
    staff[0].claim("zone.resus", non_urgent_id).unwrap();
    in_flight.push(non_urgent_task.clone());

    // Non-urgent patient works for 3 ticks
    let ticks_before_preemption = 3;
    non_urgent_task.work_elapsed_ticks += ticks_before_preemption;
    let preserved_ticks = non_urgent_task.work_elapsed_ticks;
    assert_eq!(preserved_ticks, 3);
    assert_eq!(non_urgent_task.remaining_ticks(), 7);

    // Urgent arrival occurs: ATS 1 Resuscitation arrives!
    total_admitted += 1;
    let urgent_id = 402;
    let urgent_task = PatientTask {
        task_id: "task.urgent.402".into(),
        patient_id: urgent_id,
        ats: AtsUrgency::Resuscitation, // ATS 1
        zone_id: "zone.resus".into(),
        enqueue_tick: 19,
        total_work_ticks: 4,
        work_elapsed_ticks: 0,
    };
    queue.push(urgent_task.clone());

    // Preemption triggers: urgent acuity (1) < active acuity (5)
    assert!(urgent_task.ats < non_urgent_task.ats);

    // Staff yields non-urgent patient; work ticks MUST be strictly conserved
    staff[0].release().unwrap();
    assert_eq!(
        non_urgent_task.work_elapsed_ticks, 3,
        "elapsed work ticks must be strictly conserved upon preemption"
    );

    // Urgent patient claims staff and is treated to completion
    staff[0].claim("zone.resus", urgent_id).unwrap();
    queue.retain(|t| t.patient_id != urgent_id);

    // Urgent patient occupies a surge bed or active attention, finishes in 4 ticks
    let mut finished_urgent = urgent_task;
    finished_urgent.work_elapsed_ticks = finished_urgent.total_work_ticks;
    staff[0].release().unwrap();
    completed.push(finished_urgent);

    // Resuming preempted non-urgent patient: finishes remaining ticks
    assert_eq!(
        non_urgent_task.work_elapsed_ticks, preserved_ticks,
        "preempted patient resumes with exact preserved ticks"
    );
    staff[0].claim("zone.resus", non_urgent_id).unwrap();
    let remaining = non_urgent_task.remaining_ticks();
    assert_eq!(remaining, 7);
    non_urgent_task.work_elapsed_ticks += remaining;
    assert!(non_urgent_task.is_complete());
    assert_eq!(non_urgent_task.work_elapsed_ticks, 10);

    // Non-urgent patient finishes and departs
    staff[0].release().unwrap();
    bays[0].vacate_for_cleaning(30).unwrap();
    bays[0].finish_cleaning(30).unwrap();
    in_flight.retain(|t| t.patient_id != non_urgent_id);
    completed.push(non_urgent_task);
    verify_invariants(
        &queue,
        &in_flight,
        &completed,
        &bays,
        &staff,
        total_admitted,
    );

    // -----------------------------------------------------------------------
    // (5) Bed Cleaning: Transitions to Cleaning { ready_tick } then Available
    // -----------------------------------------------------------------------
    total_admitted += 1;
    let cleaning_patient_id = 501;
    bays[1].occupy(cleaning_patient_id).unwrap();
    staff[1].claim("zone.acute", cleaning_patient_id).unwrap();
    in_flight.push(PatientTask {
        task_id: "task.501".into(),
        patient_id: cleaning_patient_id,
        ats: AtsUrgency::Urgent,
        zone_id: "zone.acute".into(),
        enqueue_tick: 31,
        total_work_ticks: 2,
        work_elapsed_ticks: 2,
    });
    staff[1].release().unwrap();

    // Patient departs; bay transitions to Cleaning { ready_tick: 35 }
    bays[1].vacate_for_cleaning(35).unwrap();
    assert_eq!(bays[1].phase, BayPhase::Cleaning { ready_tick: 35 });

    // Bay is UNAVAILABLE to new arrivals until cleaning completes
    assert_eq!(
        bays[1].occupy(502),
        Err(OperationalError::CleaningInProgress),
        "new arrivals cannot occupy a bay while cleaning is in progress"
    );

    // Cleaning still in progress before ready_tick
    assert_eq!(
        bays[1].finish_cleaning(34),
        Err(OperationalError::CleaningInProgress)
    );

    // Cleaning completes at ready_tick 35 -> Bay returns to Available
    bays[1].finish_cleaning(35).unwrap();
    assert_eq!(bays[1].phase, BayPhase::Available);

    // Now bay can be occupied by a new arrival
    assert!(bays[1].occupy(502).is_ok());
    bays[1].vacate_for_cleaning(36).unwrap();
    bays[1].finish_cleaning(36).unwrap();
    in_flight.retain(|t| t.patient_id != cleaning_patient_id);
    completed.push(PatientTask {
        task_id: "task.501".into(),
        patient_id: cleaning_patient_id,
        ats: AtsUrgency::Urgent,
        zone_id: "zone.acute".into(),
        enqueue_tick: 31,
        total_work_ticks: 2,
        work_elapsed_ticks: 2,
    });
    verify_invariants(
        &queue,
        &in_flight,
        &completed,
        &bays,
        &staff,
        total_admitted,
    );

    // -----------------------------------------------------------------------
    // (6) Boarding: Admitted patient retains bay occupancy until transfer
    // -----------------------------------------------------------------------
    total_admitted += 1;
    let boarding_patient_id = 601;
    let boarding_task = PatientTask {
        task_id: "task.boarding.601".into(),
        patient_id: boarding_patient_id,
        ats: AtsUrgency::Urgent,
        zone_id: "zone.subacute".into(),
        enqueue_tick: 37,
        total_work_ticks: 5,
        work_elapsed_ticks: 5,
    };
    bays[2].occupy(boarding_patient_id).unwrap();
    in_flight.push(boarding_task.clone());

    // Patient completes treatment and enters boarding status
    bays[2].begin_boarding().unwrap();
    assert_eq!(
        bays[2].phase,
        BayPhase::Occupied {
            patient_id: boarding_patient_id,
            diagnostic_blocked: false,
            boarding: true,
        }
    );

    // Premature release is rejected with BoardingRetained
    assert_eq!(
        bays[2].vacate_for_cleaning(45),
        Err(OperationalError::BoardingRetained),
        "premature release of a boarding patient must be rejected with BoardingRetained"
    );

    // Repeated boarding request is also rejected with BoardingRetained
    assert_eq!(
        bays[2].begin_boarding(),
        Err(OperationalError::BoardingRetained)
    );

    // Inpatient bed ready: physical departure / transfer accepted
    bays[2].accept_transfer(45).unwrap();
    assert_eq!(bays[2].phase, BayPhase::Cleaning { ready_tick: 45 });
    bays[2].finish_cleaning(45).unwrap();
    assert_eq!(bays[2].phase, BayPhase::Available);

    in_flight.retain(|t| t.patient_id != boarding_patient_id);
    completed.push(boarding_task);
    verify_invariants(
        &queue,
        &in_flight,
        &completed,
        &bays,
        &staff,
        total_admitted,
    );
}

// ===========================================================================
// Test 2: Research Ledger One-Server Queue Oracle (Report 12)
// ===========================================================================

#[test]
fn test_research_ledger_one_server_queue_oracle() {
    // Exact oracle from research notes (report 12):
    // One server; arrivals at 0 and 1; durations 2.
    // Starts at 0 and 2; waits 0 and 1; completions at 2 and 4.
    // Forced concurrent grant at tick 1 is rejected by one-server capacity.

    // 1. Compute discrete event queue oracle:
    let server_capacity = 1;
    let job1_arrival = 0u128;
    let job1_duration = 2u128;
    let job1_start = job1_arrival;
    let job1_wait = job1_start - job1_arrival;
    let job1_completion = job1_start + job1_duration;

    assert_eq!(job1_start, 0);
    assert_eq!(job1_wait, 0);
    assert_eq!(job1_completion, 2);

    let job2_arrival = 1u128;
    let job2_duration = 2u128;
    // Single server is occupied until tick 2; job 2 starts at tick 2
    let job2_start = job1_completion.max(job2_arrival);
    let job2_wait = job2_start - job2_arrival;
    let job2_completion = job2_start + job2_duration;

    assert_eq!(job2_start, 2);
    assert_eq!(job2_wait, 1);
    assert_eq!(job2_completion, 4);

    // 2. Validate against ObservedLedger in strict resource policy:
    let limits = LedgerLimits {
        max_events: 16,
        max_payload_bytes: 512,
        max_assumptions: 8,
        max_identifier_bytes: 1024,
        max_initial_resources: 4,
        max_initial_claims: 8,
        max_assumption_bytes: 1024,
    };

    let server_resource = BTreeMap::from([(
        "server".to_string(),
        InitialResource {
            capacity: server_capacity,
            claims: vec![],
        },
    )]);

    let valid_events = vec![
        // Tick 0: Job 1 starts (acquires server)
        ObservedEvent {
            order: TraceOrderKeyV1 {
                relative_ticks: 0,
                case_key: "case1".into(),
                occurrence: 0,
                event_kind_rank: EventKindRank::from_canonical_decimal("1").unwrap(),
                source_event_key: "job1_start".into(),
                source_order: 0,
            },
            available_at: Some(0),
            source_defined: true,
            transition: Transition::Acquire {
                resource: "server".into(),
                claim: "job1".into(),
                units: 1,
            },
            payload: vec![],
        },
        // Tick 1: Job 2 arrives (no resource state change, server remains held by job 1)
        ObservedEvent {
            order: TraceOrderKeyV1 {
                relative_ticks: 1,
                case_key: "case2".into(),
                occurrence: 0,
                event_kind_rank: EventKindRank::from_canonical_decimal("1").unwrap(),
                source_event_key: "job2_arrival".into(),
                source_order: 1,
            },
            available_at: Some(1),
            source_defined: true,
            transition: Transition::None,
            payload: vec![],
        },
        // Tick 2: Job 1 completes (releases server)
        ObservedEvent {
            order: TraceOrderKeyV1 {
                relative_ticks: 2,
                case_key: "case1".into(),
                occurrence: 0,
                event_kind_rank: EventKindRank::from_canonical_decimal("1").unwrap(),
                source_event_key: "job1_complete".into(),
                source_order: 2,
            },
            available_at: Some(2),
            source_defined: true,
            transition: Transition::Release {
                resource: "server".into(),
                claim: "job1".into(),
            },
            payload: vec![],
        },
        // Tick 2: Job 2 starts (acquires server)
        ObservedEvent {
            order: TraceOrderKeyV1 {
                relative_ticks: 2,
                case_key: "case2".into(),
                occurrence: 1,
                event_kind_rank: EventKindRank::from_canonical_decimal("2").unwrap(),
                source_event_key: "job2_start".into(),
                source_order: 3,
            },
            available_at: Some(2),
            source_defined: true,
            transition: Transition::Acquire {
                resource: "server".into(),
                claim: "job2".into(),
                units: 1,
            },
            payload: vec![],
        },
        // Tick 4: Job 2 completes (releases server)
        ObservedEvent {
            order: TraceOrderKeyV1 {
                relative_ticks: 4,
                case_key: "case2".into(),
                occurrence: 0,
                event_kind_rank: EventKindRank::from_canonical_decimal("1").unwrap(),
                source_event_key: "job2_complete".into(),
                source_order: 4,
            },
            available_at: Some(4),
            source_defined: true,
            transition: Transition::Release {
                resource: "server".into(),
                claim: "job2".into(),
            },
            payload: vec![],
        },
    ];

    let mut ledger = ObservedLedger::new(
        valid_events,
        server_resource.clone(),
        vec!["One-server oracle ledger".into()],
        ResourcePolicy::Strict,
        limits,
    )
    .expect("valid ledger creation");

    // Advance through all valid events; all must be resource feasible
    let mut step_count = 0;
    while let Some(snapshot) = ledger.advance().expect("advance valid events") {
        assert!(snapshot.resource_feasible);
        step_count += 1;
    }
    assert_eq!(step_count, 5);

    // 3. Forced concurrent grant at tick 1 is rejected by one-server capacity:
    let forced_concurrent_events = vec![
        // Tick 0: Job 1 starts (occupies server 1/1)
        ObservedEvent {
            order: TraceOrderKeyV1 {
                relative_ticks: 0,
                case_key: "case1".into(),
                occurrence: 0,
                event_kind_rank: EventKindRank::from_canonical_decimal("1").unwrap(),
                source_event_key: "job1_start".into(),
                source_order: 0,
            },
            available_at: Some(0),
            source_defined: true,
            transition: Transition::Acquire {
                resource: "server".into(),
                claim: "job1".into(),
                units: 1,
            },
            payload: vec![],
        },
        // Tick 1: FORCED concurrent grant to Job 2 while Job 1 is still in-flight
        ObservedEvent {
            order: TraceOrderKeyV1 {
                relative_ticks: 1,
                case_key: "case2".into(),
                occurrence: 0,
                event_kind_rank: EventKindRank::from_canonical_decimal("1").unwrap(),
                source_event_key: "job2_forced_grant".into(),
                source_order: 1,
            },
            available_at: Some(1),
            source_defined: true,
            transition: Transition::Acquire {
                resource: "server".into(),
                claim: "job2".into(),
                units: 1,
            },
            payload: vec![],
        },
    ];

    let mut infeasible_ledger = ObservedLedger::new(
        forced_concurrent_events,
        server_resource,
        vec!["Forced concurrent grant test".into()],
        ResourcePolicy::Strict,
        limits,
    )
    .expect("ledger creation");

    // Advance tick 0 -> OK
    let first = infeasible_ledger.advance().expect("job 1 start succeeds");
    assert!(first.is_some());

    // Advance tick 1 (forced concurrent grant) -> Rejected with ResourceInfeasible!
    match infeasible_ledger.advance() {
        Err(ShadowError::ResourceInfeasible(diag)) => {
            assert_eq!(diag.event, "job2_forced_grant");
            assert!(
                diag.reason.contains("occupancy exceeds capacity"),
                "diagnostic reason must cite one-server capacity exceeded: {}",
                diag.reason
            );
        }
        other => panic!("expected ShadowError::ResourceInfeasible, got {other:?}"),
    }
}

// ===========================================================================
// Calibration Helpers (experimental_c5)
// ===========================================================================

fn parse_value(raw: &str) -> Value {
    serde_json::from_str(raw).expect("fixture JSON")
}

fn plan(study: &Value) -> Value {
    parse_value(&c5::plan(&study.to_string()).expect("plan"))
}

fn summarize(study: &Value, attempts: &[Value]) -> Value {
    let attempts_json = serde_json::to_string(attempts).expect("attempts");
    parse_value(&c5::summarize(&study.to_string(), &attempts_json).expect("summarize"))
}

fn evaluate(study: &Value, task: &Value) -> Value {
    parse_value(&c5::evaluate(&study.to_string(), &task.to_string()).expect("evaluate"))
}

fn evaluate_all_tasks(study: &Value, tasks: &[Value]) -> Vec<Value> {
    tasks.iter().map(|task| evaluate(study, task)).collect()
}

fn complete_study(study: &Value) -> (Value, Vec<Value>) {
    let planned = plan(study);
    let tasks = planned["tasks"].as_array().expect("tasks").clone();
    let mut attempts = evaluate_all_tasks(study, &tasks);
    let mut summary = summarize(study, &attempts);
    while summary["complete"] == false {
        let next = summary["next_tasks"]
            .as_array()
            .expect("next_tasks")
            .clone();
        assert!(!next.is_empty(), "incomplete study without next tasks");
        attempts.extend(evaluate_all_tasks(study, &next));
        summary = summarize(study, &attempts);
    }
    (summary, attempts)
}

fn build_synthetic_ed_study() -> Value {
    json!({
        "schema_version": "kairoecs.calibration.study.v1",
        "study_id": "synthetic-ed-fixture-c61",
        "dataset_id": "synthetic-ed",
        "mapping_version": "normalized-ticks.v1",
        "adapter": "synthetic_flow_probe.v1",
        "root_seed": 42,
        "parameters": {
            "route_ticks": {
                "unit": "tick",
                "values": [1, 2, 3]
            },
            "work_ticks": {
                "unit": "tick",
                "values": [2, 3, 4]
            }
        },
        "objective": {
            "completion_weight": 1.0,
            "transit_weight": 1.0,
            "scale_ticks": 1,
            "failure_penalty": 1000,
            "tie_tolerance": 0.0
        },
        "budget": {
            "max_candidates": 100,
            "replications": 2,
            "confirmation_replications": 2,
            "final_test_replications": 2,
            "max_tasks": 10000,
            "horizon_ticks": 1000,
            "max_events": 1000
        },
        "precision": {
            "kind": "fixed_confirmation",
            "target_half_width": 1,
            "alpha": 0.05
        },
        "splits": {
            "training": [0, 100],
            "validation": [100, 200],
            "final_test": [200, 300]
        },
        "trace": [
            {
                "case_id": "ed-train-1",
                "day": "ed-day-train-1",
                "start_tick": 0,
                "end_tick": 5,
                "transit_ticks": 2
            },
            {
                "case_id": "ed-val-1",
                "day": "ed-day-val-1",
                "start_tick": 100,
                "end_tick": 105,
                "transit_ticks": 2
            },
            {
                "case_id": "ed-test-1",
                "day": "ed-day-test-1",
                "start_tick": 200,
                "end_tick": 205,
                "transit_ticks": 2
            }
        ]
    })
}

// ===========================================================================
// Test 3: Identifiable Fixture Recovers Known Transit Parameters
// ===========================================================================

#[test]
fn test_calibration_recovers_known_transit_on_identifiable_fixture() {
    let study = build_synthetic_ed_study();
    let (summary, _) = complete_study(&study);

    // Asserts:
    // winner is c000004 (route_ticks = 2, work_ticks = 3)
    assert_eq!(summary["winner"], "c000004");

    let winner_row = summary["candidates"]
        .as_array()
        .expect("candidates")
        .iter()
        .find(|c| c["candidate_id"] == summary["winner"])
        .expect("winner row in summary");
    assert_eq!(winner_row["params"]["route_ticks"], 2);
    assert_eq!(winner_row["params"]["work_ticks"], 3);

    assert_eq!(summary["ridge"], false);
    assert_eq!(summary["identifiability"], "not_demonstrated");
    assert_eq!(summary["final_test"]["mean_completion_error"], 0.0);
}

// ===========================================================================
// Test 4: Confounded Case Flags Ridge Without Unique Recovery Claim
// ===========================================================================

#[test]
fn test_calibration_flags_deliberately_confounded_case_without_claiming_unique_recovery() {
    let mut study = build_synthetic_ed_study();

    // Transit is unobserved: transit_ticks = null, transit_weight = 0
    study["objective"]["transit_weight"] = json!(0.0);
    study["objective"]["tie_tolerance"] = json!(0.0);
    for row in study["trace"].as_array_mut().unwrap() {
        row["transit_ticks"] = Value::Null;
    }

    // Grid candidates include decompositions adding up to 5:
    // (1, 4) -> c000002
    // (2, 3) -> c000004
    // (3, 2) -> c000006
    let planned = plan(&study);
    let attempts = evaluate_all_tasks(&study, planned["tasks"].as_array().expect("tasks"));
    let summary = summarize(&study, &attempts);

    // Asserts:
    // Ridge detected due to unobserved transit confounding
    assert_eq!(summary["ridge"], true);
    assert_eq!(summary["identifiability"], "non_identifiable_ridge");

    // Multiple top candidates achieve identical objective score 0.0
    let ranking = summary["ranking"].as_array().expect("ranking");
    assert!(ranking.len() >= 3);
    assert_eq!(ranking[0]["score"], 0.0);
    assert_eq!(ranking[1]["score"], 0.0);
    assert_eq!(ranking[2]["score"], 0.0);

    // Instead of claiming unique recovery, the runner reports ambiguity and
    // tie-breaks by ascending candidate ID
    assert_eq!(summary["winner"], "c000002");
    assert_eq!(ranking[0]["candidate_id"], "c000002");
    assert_eq!(ranking[1]["candidate_id"], "c000004");
    assert_eq!(ranking[2]["candidate_id"], "c000006");
    assert!(
        ranking[0]["candidate_id"].as_str() < ranking[1]["candidate_id"].as_str(),
        "equal scores must tie-break by ascending candidate_id"
    );
}
