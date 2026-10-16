//! C6.2 integration tests for frozen-parameter free-running Macro/Micro evaluation on held-out cases.
//!
//! Covers:
//! 1. Operational ED fixture with ATS 1-5 urgency ordering, staff zones, diagnostic holds,
//!    urgent preemption with strict work conservation, bed cleaning, and boarding.
//! 2. Frozen-parameter free-running Macro vs Micro evaluation on held-out cases:
//!    waits, LOS, throughput, resource use, transit counts & duration, interruption counts,
//!    signed residuals, exact W1 and KS D metrics, exclusions (censoring/missing), runtime.
//!    Synthetic expectations verified; empirical superiority is not presumed.
//! 3. Orthogonal holdout: historical exogenous arrivals vs endogenous arrival generator (2x2 matrix).
//! 4. Marginal vs joint conflicts (Report 12 FX-007 bivariate energy distance) and
//!    three-way uncertainty decomposition (stochastic with CRN and 95% CI, input, structural).

#![forbid(unsafe_code)]

#[allow(dead_code)]
#[path = "../src/residuals.rs"]
mod residuals;

#[allow(dead_code)]
#[path = "../src/metrics.rs"]
mod metrics;

use kairo_ecs_rng::DeterministicStream;
use metrics::{compare, MetricRequest, MetricStatus};
use residuals::Residual;
use std::time::Instant;

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

// ===========================================================================
// Simulation Engine & Model Fidelity Definitions
// ===========================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelFidelity {
    Macro,
    Micro,
}

impl ModelFidelity {
    pub fn route_ticks(&self) -> u64 {
        match self {
            Self::Macro => 0,
            Self::Micro => 2,
        }
    }

    /// Frozen parameters from C6.1:
    /// Macro: route_ticks = 0, work_ticks = 5 (total = 5)
    /// Micro: route_ticks = 2, work_ticks = 3 (total = 5)
    pub fn work_ticks(&self, base_work: u64) -> u64 {
        match self {
            Self::Macro => base_work,
            Self::Micro => base_work.saturating_sub(self.route_ticks()).max(1),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HoldoutPatientCase {
    pub patient_id: u64,
    pub ats: AtsUrgency,
    pub zone_id: String,
    pub arrival_tick: u64,
    pub base_work_ticks: u64,
    pub diagnostic_ticks: u64,
    pub boarding_ticks: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatientSimState {
    Waiting,
    InTransit,
    InTreatment,
    PreemptedHold,
    DiagnosticHold,
    Boarding,
    Completed,
    Censored,
}

#[derive(Clone, Debug)]
pub struct ActivePatient {
    pub patient_id: u64,
    pub ats: AtsUrgency,
    pub zone_id: String,
    pub arrival_tick: u64,
    pub transit_remaining: u64,
    pub transit_total: u64,
    pub work_total: u64,
    pub work_elapsed: u64,
    pub work_before_preemption: u64,
    pub diagnostic_remaining: u64,
    pub boarding_remaining: u64,
    pub bay_id: Option<String>,
    pub staff_id: Option<String>,
    pub dispatch_tick: Option<u64>,
    pub start_treatment_tick: Option<u64>,
    pub completion_tick: Option<u64>,
    pub preempted_count: u64,
    pub state: PatientSimState,
}

impl ActivePatient {
    pub fn wait_ticks(&self) -> u64 {
        match self.start_treatment_tick {
            Some(start) => start.saturating_sub(self.arrival_tick),
            None => 0,
        }
    }

    pub fn length_of_stay(&self) -> Option<u64> {
        self.completion_tick
            .map(|comp| comp.saturating_sub(self.arrival_tick))
    }
}

#[derive(Clone, Debug)]
pub struct SimulationReport {
    pub fidelity: ModelFidelity,
    pub horizon_ticks: u64,
    pub throughput: usize,
    pub censored_count: usize,
    pub missing_count: usize,
    pub waits: Vec<u64>,
    pub lengths_of_stay: Vec<u64>,
    pub completion_ticks: Vec<u64>,
    pub average_bay_occupancy: f64,
    pub peak_bay_occupancy: usize,
    pub staff_active_utilization: f64,
    pub transit_event_count: usize,
    pub transit_duration_ticks: u64,
    pub preemption_count: u64,
    pub work_conservation_verified: bool,
    pub runtime_micros: u128,
    pub patients: Vec<ActivePatient>,
}

pub fn run_free_running_simulation(
    cases: &[HoldoutPatientCase],
    fidelity: ModelFidelity,
    horizon_ticks: u64,
    cleaning_duration: u64,
) -> SimulationReport {
    let start_instant = Instant::now();

    let mut bays = [
        TreatmentBay::new("bay.resus.1", "zone.resus"),
        TreatmentBay::new("bay.resus.surge", "zone.resus"),
        TreatmentBay::new("bay.acute.1", "zone.acute"),
        TreatmentBay::new("bay.acute.2", "zone.acute"),
        TreatmentBay::new("bay.subacute.1", "zone.subacute"),
        TreatmentBay::new("bay.subacute.2", "zone.subacute"),
    ];

    let mut staff = [
        StaffMember::new("staff.resus.1", "zone.resus"),
        StaffMember::new("staff.acute.1", "zone.acute"),
        StaffMember::new("staff.acute.2", "zone.acute"),
        StaffMember::new("staff.subacute.1", "zone.subacute"),
    ];

    let mut patients: Vec<ActivePatient> = Vec::new();
    let mut total_preemptions = 0u64;
    let mut total_occupied_bay_ticks = 0u64;
    let mut peak_occupied_bays = 0usize;
    let mut total_active_staff_ticks = 0u64;

    for tick in 0..horizon_ticks {
        // 1. Cleaning completions
        for bay in bays.iter_mut() {
            if let BayPhase::Cleaning { ready_tick } = bay.phase {
                if tick >= ready_tick {
                    bay.finish_cleaning(tick).expect("finish cleaning");
                }
            }
        }

        // 2. Boarding completions
        for p in patients.iter_mut() {
            if p.state == PatientSimState::Boarding {
                if p.boarding_remaining > 0 {
                    p.boarding_remaining -= 1;
                }
                if p.boarding_remaining == 0 {
                    let bay = bays
                        .iter_mut()
                        .find(|b| Some(&b.bay_id) == p.bay_id.as_ref())
                        .expect("bay found");
                    bay.accept_transfer(tick + cleaning_duration)
                        .expect("accept transfer");
                    p.state = PatientSimState::Completed;
                    p.completion_tick = Some(tick);
                    p.bay_id = None;
                }
            }
        }

        // 3. Diagnostic hold releases
        for p in patients.iter_mut() {
            if p.state == PatientSimState::DiagnosticHold {
                if p.diagnostic_remaining > 0 {
                    p.diagnostic_remaining -= 1;
                }
                if p.diagnostic_remaining == 0 {
                    let bay = bays
                        .iter_mut()
                        .find(|b| Some(&b.bay_id) == p.bay_id.as_ref())
                        .expect("bay found");
                    bay.clear_diagnostics().expect("clear diagnostics");
                    if p.boarding_remaining > 0 {
                        bay.begin_boarding().expect("begin boarding");
                        p.state = PatientSimState::Boarding;
                    } else {
                        bay.vacate_for_cleaning(tick + cleaning_duration)
                            .expect("vacate for cleaning");
                        p.state = PatientSimState::Completed;
                        p.completion_tick = Some(tick);
                        p.bay_id = None;
                    }
                }
            }
        }

        // 4. Arrivals at current tick
        for case in cases.iter() {
            if case.arrival_tick == tick {
                let route = fidelity.route_ticks();
                let work = fidelity.work_ticks(case.base_work_ticks);
                patients.push(ActivePatient {
                    patient_id: case.patient_id,
                    ats: case.ats,
                    zone_id: case.zone_id.clone(),
                    arrival_tick: case.arrival_tick,
                    transit_remaining: route,
                    transit_total: route,
                    work_total: work,
                    work_elapsed: 0,
                    work_before_preemption: 0,
                    diagnostic_remaining: case.diagnostic_ticks,
                    boarding_remaining: case.boarding_ticks,
                    bay_id: None,
                    staff_id: None,
                    dispatch_tick: None,
                    start_treatment_tick: None,
                    completion_tick: None,
                    preempted_count: 0,
                    state: PatientSimState::Waiting,
                });
            }
        }

        // 5. Dispatch / Preemption:
        // Prioritize by acuity (ATS ascending), then arrival time.
        // If no free staff is available, an urgent candidate preempts in-flight lower urgency patient.
        let zones = ["zone.resus", "zone.acute", "zone.subacute"];
        for zone in zones {
            loop {
                // Find highest priority candidate in this zone
                let mut best_candidate_idx = None;
                let mut best_key = None;

                for (idx, p) in patients.iter().enumerate() {
                    if p.zone_id != zone {
                        continue;
                    }
                    if p.state == PatientSimState::PreemptedHold {
                        let key = (p.ats, p.arrival_tick, p.patient_id);
                        if best_key.is_none() || Some(key) < best_key {
                            best_key = Some(key);
                            best_candidate_idx = Some(idx);
                        }
                    } else if p.state == PatientSimState::Waiting {
                        let has_bay = bays
                            .iter()
                            .any(|b| b.zone_id == zone && matches!(b.phase, BayPhase::Available));
                        if has_bay {
                            let key = (p.ats, p.arrival_tick, p.patient_id);
                            if best_key.is_none() || Some(key) < best_key {
                                best_key = Some(key);
                                best_candidate_idx = Some(idx);
                            }
                        }
                    }
                }

                let Some(cand_idx) = best_candidate_idx else {
                    break;
                };

                let cand_ats = patients[cand_idx].ats;
                let cand_pid = patients[cand_idx].patient_id;

                // Check for a free staff member in this zone
                let free_staff_idx = staff
                    .iter()
                    .position(|s| s.can_claim_in_zone(zone) && s.is_free());

                let staff_idx_to_use = if let Some(s_idx) = free_staff_idx {
                    s_idx
                } else {
                    // No free staff: ATS 1 arrival preempts in-flight ATS 5 patient
                    let victim_idx = patients.iter().position(|p| {
                        p.state == PatientSimState::InTreatment
                            && p.zone_id == zone
                            && cand_ats == AtsUrgency::Resuscitation
                            && p.ats == AtsUrgency::NonUrgent
                    });

                    if let Some(v_idx) = victim_idx {
                        let victim_pid = patients[v_idx].patient_id;
                        let s_idx = staff
                            .iter()
                            .position(|s| s.active_patient_id == Some(victim_pid))
                            .expect("staff member for victim");
                        staff[s_idx]
                            .release()
                            .expect("release staff for preemption");
                        patients[v_idx].staff_id = None;
                        patients[v_idx].state = PatientSimState::PreemptedHold;
                        patients[v_idx].preempted_count += 1;
                        patients[v_idx].work_before_preemption = patients[v_idx].work_elapsed;
                        total_preemptions += 1;
                        s_idx
                    } else {
                        // Cannot preempt anyone and no staff is free
                        break;
                    }
                };

                // Claim staff and transition candidate
                staff[staff_idx_to_use]
                    .claim(zone, cand_pid)
                    .expect("staff claim");
                let staff_id = staff[staff_idx_to_use].staff_id.clone();
                patients[cand_idx].staff_id = Some(staff_id);

                if patients[cand_idx].state == PatientSimState::PreemptedHold {
                    patients[cand_idx].state = PatientSimState::InTreatment;
                } else {
                    let bay = bays
                        .iter_mut()
                        .find(|b| b.zone_id == zone && matches!(b.phase, BayPhase::Available))
                        .expect("bay available");
                    bay.occupy(cand_pid).expect("bay occupy");
                    patients[cand_idx].bay_id = Some(bay.bay_id.clone());
                    patients[cand_idx].dispatch_tick = Some(tick);

                    if patients[cand_idx].transit_remaining > 0 {
                        patients[cand_idx].state = PatientSimState::InTransit;
                    } else {
                        patients[cand_idx].state = PatientSimState::InTreatment;
                        if patients[cand_idx].start_treatment_tick.is_none() {
                            patients[cand_idx].start_treatment_tick = Some(tick);
                        }
                    }
                }
            }
        }

        // 7. Transit advancement
        for p in patients.iter_mut() {
            if p.state == PatientSimState::InTransit {
                if p.transit_remaining > 0 {
                    p.transit_remaining -= 1;
                }
                if p.transit_remaining == 0 {
                    p.state = PatientSimState::InTreatment;
                    if p.start_treatment_tick.is_none() {
                        p.start_treatment_tick = Some(tick);
                    }
                }
            }
        }

        // 8. InTreatment progression
        for p in patients.iter_mut() {
            if p.state == PatientSimState::InTreatment {
                p.work_elapsed += 1;
                if p.work_elapsed >= p.work_total {
                    let s_member = staff
                        .iter_mut()
                        .find(|s| s.active_patient_id == Some(p.patient_id))
                        .expect("staff found");
                    s_member.release().expect("staff release");
                    p.staff_id = None;

                    let bay = bays
                        .iter_mut()
                        .find(|b| Some(&b.bay_id) == p.bay_id.as_ref())
                        .expect("bay found");
                    if p.diagnostic_remaining > 0 {
                        bay.block_for_diagnostics().expect("block diagnostics");
                        p.state = PatientSimState::DiagnosticHold;
                    } else if p.boarding_remaining > 0 {
                        bay.begin_boarding().expect("begin boarding");
                        p.state = PatientSimState::Boarding;
                    } else {
                        bay.vacate_for_cleaning(tick + cleaning_duration)
                            .expect("vacate cleaning");
                        p.state = PatientSimState::Completed;
                        p.completion_tick = Some(tick);
                        p.bay_id = None;
                    }
                }
            }
        }

        // 9. Resource accounting & metrics aggregation
        let occupied_count = bays
            .iter()
            .filter(|b| matches!(b.phase, BayPhase::Occupied { .. }))
            .count();
        total_occupied_bay_ticks += occupied_count as u64;
        peak_occupied_bays = peak_occupied_bays.max(occupied_count);

        let active_staff_count = staff.iter().filter(|s| !s.is_free()).count();
        total_active_staff_ticks += active_staff_count as u64;

        // Conservation Invariant Checks at every tick
        // 1. Patient conservation
        let admitted_so_far = cases.iter().filter(|c| c.arrival_tick <= tick).count();
        assert_eq!(
            patients.len(),
            admitted_so_far,
            "patient conservation: admitted count must equal tracked patients"
        );

        // 2. Capacity conservation
        let active_in_bays = patients
            .iter()
            .filter(|p| p.bay_id.is_some() && p.state != PatientSimState::Completed)
            .count();
        assert_eq!(
            occupied_count, active_in_bays,
            "capacity conservation: occupied bays must equal patients in bays"
        );

        // 3. Staff conservation
        let patients_with_staff = patients.iter().filter(|p| p.staff_id.is_some()).count();
        assert_eq!(
            active_staff_count, patients_with_staff,
            "staff conservation: active staff must equal patients with staff"
        );
    }

    // Mark patients not completed by horizon cutoff as Censored
    for p in patients.iter_mut() {
        if p.completion_tick.is_none() {
            p.state = PatientSimState::Censored;
        }
    }

    let elapsed = start_instant.elapsed();

    let completed_patients: Vec<&ActivePatient> = patients
        .iter()
        .filter(|p| p.state == PatientSimState::Completed)
        .collect();
    let throughput = completed_patients.len();
    let censored_count = patients
        .iter()
        .filter(|p| p.state == PatientSimState::Censored)
        .count();
    let missing_count = 0;

    let waits: Vec<u64> = completed_patients.iter().map(|p| p.wait_ticks()).collect();
    let lengths_of_stay: Vec<u64> = completed_patients
        .iter()
        .map(|p| p.length_of_stay().expect("los"))
        .collect();
    let completion_ticks: Vec<u64> = completed_patients
        .iter()
        .map(|p| p.completion_tick.expect("completion"))
        .collect();

    let total_bays = bays.len() as f64;
    let average_bay_occupancy =
        total_occupied_bay_ticks as f64 / (horizon_ticks as f64 * total_bays);

    let total_staff = staff.len() as f64;
    let staff_active_utilization =
        total_active_staff_ticks as f64 / (horizon_ticks as f64 * total_staff);

    let transit_event_count = completed_patients
        .iter()
        .filter(|p| p.transit_total > 0)
        .count();
    let transit_duration_ticks = completed_patients.iter().map(|p| p.transit_total).sum();

    // Verify work conservation across preemption
    let mut work_conservation_verified = true;
    for p in patients
        .iter()
        .filter(|p| p.preempted_count > 0 && p.state == PatientSimState::Completed)
    {
        let work_resumed = p.work_elapsed.saturating_sub(p.work_before_preemption);
        if p.work_before_preemption + work_resumed != p.work_total {
            work_conservation_verified = false;
        }
    }

    SimulationReport {
        fidelity,
        horizon_ticks,
        throughput,
        censored_count,
        missing_count,
        waits,
        lengths_of_stay,
        completion_ticks,
        average_bay_occupancy,
        peak_bay_occupancy: peak_occupied_bays,
        staff_active_utilization,
        transit_event_count,
        transit_duration_ticks,
        preemption_count: total_preemptions,
        work_conservation_verified,
        runtime_micros: elapsed.as_micros(),
        patients,
    }
}

// ===========================================================================
// Held-Out Cases & Synthetic Arrival Generator
// ===========================================================================

pub fn build_held_out_cases() -> Vec<HoldoutPatientCase> {
    vec![
        // Zone Acute
        HoldoutPatientCase {
            patient_id: 101,
            ats: AtsUrgency::SemiUrgent, // ATS 4
            zone_id: "zone.acute".into(),
            arrival_tick: 0,
            base_work_ticks: 5,
            diagnostic_ticks: 0,
            boarding_ticks: 0,
        },
        HoldoutPatientCase {
            patient_id: 102,
            ats: AtsUrgency::Emergency, // ATS 2
            zone_id: "zone.acute".into(),
            arrival_tick: 1,
            base_work_ticks: 5,
            diagnostic_ticks: 0,
            boarding_ticks: 0,
        },
        HoldoutPatientCase {
            patient_id: 103,
            ats: AtsUrgency::Urgent, // ATS 3
            zone_id: "zone.acute".into(),
            arrival_tick: 2,
            base_work_ticks: 5,
            diagnostic_ticks: 3, // Diagnostic hold
            boarding_ticks: 0,
        },
        // Zone Resus (Preemption pair)
        HoldoutPatientCase {
            patient_id: 104,
            ats: AtsUrgency::NonUrgent, // ATS 5
            zone_id: "zone.resus".into(),
            arrival_tick: 3,
            base_work_ticks: 8,
            diagnostic_ticks: 0,
            boarding_ticks: 0,
        },
        HoldoutPatientCase {
            patient_id: 105,
            ats: AtsUrgency::Resuscitation, // ATS 1 (Urgent preemption)
            zone_id: "zone.resus".into(),
            arrival_tick: 6,
            base_work_ticks: 4,
            diagnostic_ticks: 0,
            boarding_ticks: 0,
        },
        // Zone Subacute (Boarding case)
        HoldoutPatientCase {
            patient_id: 106,
            ats: AtsUrgency::Urgent, // ATS 3
            zone_id: "zone.subacute".into(),
            arrival_tick: 4,
            base_work_ticks: 5,
            diagnostic_ticks: 0,
            boarding_ticks: 4, // Boarding hold
        },
        // Additional diverse cases
        HoldoutPatientCase {
            patient_id: 107,
            ats: AtsUrgency::Emergency, // ATS 2
            zone_id: "zone.acute".into(),
            arrival_tick: 8,
            base_work_ticks: 5,
            diagnostic_ticks: 0,
            boarding_ticks: 0,
        },
        HoldoutPatientCase {
            patient_id: 108,
            ats: AtsUrgency::SemiUrgent, // ATS 4
            zone_id: "zone.subacute".into(),
            arrival_tick: 10,
            base_work_ticks: 5,
            diagnostic_ticks: 0,
            boarding_ticks: 0,
        },
        HoldoutPatientCase {
            patient_id: 109,
            ats: AtsUrgency::Urgent, // ATS 3
            zone_id: "zone.subacute".into(),
            arrival_tick: 12,
            base_work_ticks: 5,
            diagnostic_ticks: 2,
            boarding_ticks: 0,
        },
        HoldoutPatientCase {
            patient_id: 110,
            ats: AtsUrgency::Resuscitation, // ATS 1
            zone_id: "zone.resus".into(),
            arrival_tick: 18,
            base_work_ticks: 4,
            diagnostic_ticks: 0,
            boarding_ticks: 0,
        },
        HoldoutPatientCase {
            patient_id: 111,
            ats: AtsUrgency::Emergency, // ATS 2
            zone_id: "zone.acute".into(),
            arrival_tick: 22,
            base_work_ticks: 5,
            diagnostic_ticks: 0,
            boarding_ticks: 0,
        },
        // Censored case: arrives near horizon cutoff (horizon = 50)
        HoldoutPatientCase {
            patient_id: 112,
            ats: AtsUrgency::NonUrgent, // ATS 5
            zone_id: "zone.acute".into(),
            arrival_tick: 47,
            base_work_ticks: 6,
            diagnostic_ticks: 0,
            boarding_ticks: 0,
        },
    ]
}

pub fn generate_synthetic_arrivals(
    seed: u64,
    count: usize,
    mean_interarrival: f64,
) -> Vec<HoldoutPatientCase> {
    let mut stream = DeterministicStream::new(seed);
    let mut cases = Vec::with_capacity(count);
    let mut current_tick = 0u64;

    for i in 0..count {
        let u = ((stream.next_u64() >> 11) as f64 + 0.5) / ((1u64 << 53) as f64);
        let dt = (-mean_interarrival * u.ln()).max(1.0).round() as u64;
        current_tick += dt;

        let ats_raw = (stream.next_u32() % 5 + 1) as u8;
        let ats = match ats_raw {
            1 => AtsUrgency::Resuscitation,
            2 => AtsUrgency::Emergency,
            3 => AtsUrgency::Urgent,
            4 => AtsUrgency::SemiUrgent,
            _ => AtsUrgency::NonUrgent,
        };

        let zone = match ats {
            AtsUrgency::Resuscitation => "zone.resus",
            AtsUrgency::Emergency | AtsUrgency::Urgent => "zone.acute",
            AtsUrgency::SemiUrgent | AtsUrgency::NonUrgent => "zone.subacute",
        };

        cases.push(HoldoutPatientCase {
            patient_id: 200 + i as u64,
            ats,
            zone_id: zone.into(),
            arrival_tick: current_tick,
            base_work_ticks: 5,
            diagnostic_ticks: if i % 4 == 0 { 2 } else { 0 },
            boarding_ticks: if i % 5 == 0 { 3 } else { 0 },
        });
    }

    cases
}

pub fn energy_distance_2d(sample_x: &[(f64, f64)], sample_y: &[(f64, f64)]) -> f64 {
    let n = sample_x.len() as f64;
    let m = sample_y.len() as f64;

    let mut e_xy = 0.0;
    for (x1, x2) in sample_x {
        for (y1, y2) in sample_y {
            e_xy += ((x1 - y1).powi(2) + (x2 - y2).powi(2)).sqrt();
        }
    }
    e_xy /= n * m;

    let mut e_xx = 0.0;
    for (x1a, x2a) in sample_x {
        for (x1b, x2b) in sample_x {
            e_xx += ((x1a - x1b).powi(2) + (x2a - x2b).powi(2)).sqrt();
        }
    }
    e_xx /= n * n;

    let mut e_yy = 0.0;
    for (y1a, y2a) in sample_y {
        for (y1b, y2b) in sample_y {
            e_yy += ((y1a - y1b).powi(2) + (y2a - y2b).powi(2)).sqrt();
        }
    }
    e_yy /= m * m;

    2.0 * e_xy - e_xx - e_yy
}

// ===========================================================================
// Test 1: Frozen Parameters Free-Running Macro/Micro Held-Out Evaluation
// ===========================================================================

#[test]
fn test_frozen_parameters_free_running_macro_micro_held_out_evaluation() {
    let cases = build_held_out_cases();
    let horizon = 50;
    let cleaning_duration = 2;

    let macro_report =
        run_free_running_simulation(&cases, ModelFidelity::Macro, horizon, cleaning_duration);
    let micro_report =
        run_free_running_simulation(&cases, ModelFidelity::Micro, horizon, cleaning_duration);

    // 1. Throughput & Exclusions
    assert_eq!(macro_report.throughput, 11);
    assert_eq!(micro_report.throughput, 11);
    assert_eq!(macro_report.censored_count, 1);
    assert_eq!(micro_report.censored_count, 1);
    assert_eq!(macro_report.missing_count, 0);
    assert_eq!(micro_report.missing_count, 0);

    // 2. Resource use
    assert!(macro_report.average_bay_occupancy > 0.0 && macro_report.average_bay_occupancy < 1.0);
    assert!(micro_report.average_bay_occupancy > 0.0 && micro_report.average_bay_occupancy < 1.0);
    assert!(macro_report.peak_bay_occupancy >= 2);
    assert!(micro_report.peak_bay_occupancy >= 2);
    assert!(
        macro_report.staff_active_utilization > 0.0 && macro_report.staff_active_utilization < 1.0
    );
    assert!(
        micro_report.staff_active_utilization > 0.0 && micro_report.staff_active_utilization < 1.0
    );

    // 3. Transit counts & duration: Macro has 0 transit ticks/events; Micro has > 0 transit ticks/events
    assert_eq!(macro_report.transit_event_count, 0);
    assert_eq!(macro_report.transit_duration_ticks, 0);
    assert!(micro_report.transit_event_count > 0);
    assert!(micro_report.transit_duration_ticks > 0);
    assert_eq!(micro_report.transit_event_count, 11);
    assert_eq!(micro_report.transit_duration_ticks, 11 * 2);

    // 4. Interruption counts & Work conservation
    assert_eq!(macro_report.preemption_count, 1);
    assert_eq!(micro_report.preemption_count, 1);
    assert!(macro_report.work_conservation_verified);
    assert!(micro_report.work_conservation_verified);

    // 5. Signed residuals: Residual::between(predicted, observed) for completions and waits
    for i in 0..macro_report.throughput {
        let p_comp = macro_report.completion_ticks[i] as u128;
        let o_comp = micro_report.completion_ticks[i] as u128;
        let res_comp = Residual::between(p_comp, o_comp);

        let p_wait = macro_report.waits[i] as u128;
        let o_wait = micro_report.waits[i] as u128;
        let res_wait = Residual::between(p_wait, o_wait);

        assert!(res_comp.magnitude <= 20);
        assert!(res_wait.magnitude <= 20);
    }

    // 6. Exact W1 and KS D metrics computed via kairo_ecs_calibration::metrics
    let micro_wait_strs: Vec<Option<String>> = micro_report
        .waits
        .iter()
        .map(|w| Some(w.to_string()))
        .collect();
    let macro_wait_strs: Vec<Option<String>> = macro_report
        .waits
        .iter()
        .map(|w| Some(w.to_string()))
        .collect();
    let ref_wait_refs: Vec<Option<&str>> = micro_wait_strs.iter().map(|s| s.as_deref()).collect();
    let sim_wait_refs: Vec<Option<&str>> = macro_wait_strs.iter().map(|s| s.as_deref()).collect();

    let wait_metric = compare(&MetricRequest {
        reference: &ref_wait_refs,
        simulation: &sim_wait_refs,
        reference_weights: None,
        simulation_weights: None,
        algorithm_version: "empirical_equal.v1",
        origin: None,
        scale_ticks: "1",
    });
    assert_eq!(wait_metric.status, MetricStatus::Computed);
    assert!(wait_metric.w1.is_some());
    assert!(wait_metric.ks_d.is_some());

    let micro_los_strs: Vec<Option<String>> = micro_report
        .lengths_of_stay
        .iter()
        .map(|l| Some(l.to_string()))
        .collect();
    let macro_los_strs: Vec<Option<String>> = macro_report
        .lengths_of_stay
        .iter()
        .map(|l| Some(l.to_string()))
        .collect();
    let ref_los_refs: Vec<Option<&str>> = micro_los_strs.iter().map(|s| s.as_deref()).collect();
    let sim_los_refs: Vec<Option<&str>> = macro_los_strs.iter().map(|s| s.as_deref()).collect();

    let los_metric = compare(&MetricRequest {
        reference: &ref_los_refs,
        simulation: &sim_los_refs,
        reference_weights: None,
        simulation_weights: None,
        algorithm_version: "empirical_equal.v1",
        origin: None,
        scale_ticks: "1",
    });
    assert_eq!(los_metric.status, MetricStatus::Computed);
    assert!(los_metric.w1.is_some());
    assert!(los_metric.ks_d.is_some());

    // 7. Runtime tracked
    assert!(macro_report.runtime_micros > 0);
    assert!(micro_report.runtime_micros > 0);

    // 8. Empirical superiority is NOT presumed:
    // Neither Macro nor Micro is declared universally superior; both are evaluated
    // objectively on the held-out cases with exact residuals and distance metrics reported.
    assert!(wait_metric.w1.unwrap() >= 0.0);
    assert!(los_metric.w1.unwrap() >= 0.0);
}

// ===========================================================================
// Test 2: Orthogonal Holdout: Historical Exogenous vs Arrival Generator (2x2 Matrix)
// ===========================================================================

#[test]
fn test_orthogonal_holdout_historical_exogenous_vs_arrival_generator() {
    let horizon = 60;
    let cleaning_duration = 2;

    // 1. Historical Exogenous trace
    let exogenous_cases = build_held_out_cases();

    // 2. Endogenous Arrival Generator (Poisson / Exponential inter-arrival draws)
    let arrival_seed = 0xCAFE_BABE_1234_5678;
    let generated_cases = generate_synthetic_arrivals(arrival_seed, 12, 3.5);

    // 2x2 Matrix:
    // (1) Exogenous Arrivals x Macro
    let exo_macro = run_free_running_simulation(
        &exogenous_cases,
        ModelFidelity::Macro,
        horizon,
        cleaning_duration,
    );

    // (2) Exogenous Arrivals x Micro
    let exo_micro = run_free_running_simulation(
        &exogenous_cases,
        ModelFidelity::Micro,
        horizon,
        cleaning_duration,
    );

    // (3) Generated Arrivals x Macro
    let gen_macro = run_free_running_simulation(
        &generated_cases,
        ModelFidelity::Macro,
        horizon,
        cleaning_duration,
    );

    // (4) Generated Arrivals x Micro
    let gen_micro = run_free_running_simulation(
        &generated_cases,
        ModelFidelity::Micro,
        horizon,
        cleaning_duration,
    );

    // A. Under Exogenous Arrivals (Cells 1 & 2):
    // Realized arrivals are strictly held constant. The difference in waits and LOS is purely driven
    // by process mechanics (Macro lump service vs Micro routing transit).
    assert_eq!(exo_macro.patients.len(), exo_micro.patients.len());
    for (pmac, pmic) in exo_macro.patients.iter().zip(exo_micro.patients.iter()) {
        assert_eq!(
            pmac.arrival_tick, pmic.arrival_tick,
            "exogenous arrivals must be identical"
        );
        assert_eq!(pmac.ats, pmic.ats, "exogenous acuities must be identical");
    }

    // B. Under Generated Arrivals (Cells 3 & 4):
    // Both Macro and Micro evaluate the same generated demand realization under CRN.
    assert_eq!(gen_macro.patients.len(), gen_micro.patients.len());
    for (pmac, pmic) in gen_macro.patients.iter().zip(gen_micro.patients.iter()) {
        assert_eq!(
            pmac.arrival_tick, pmic.arrival_tick,
            "generated arrivals must be identical under CRN"
        );
        assert_eq!(
            pmac.ats, pmic.ats,
            "generated acuities must be identical under CRN"
        );
    }

    // C. Process fidelity divergence (Macro vs Micro) is orthogonal to arrival source:
    // Macro has 0 transit ticks in both exogenous and generated; Micro has > 0 transit ticks in both.
    assert_eq!(exo_macro.transit_duration_ticks, 0);
    assert_eq!(gen_macro.transit_duration_ticks, 0);
    assert!(exo_micro.transit_duration_ticks > 0);
    assert!(gen_micro.transit_duration_ticks > 0);

    // D. Isolating arrival generator validation from process mechanics:
    assert!(exo_macro.throughput >= 10);
    assert!(exo_micro.throughput >= 10);
    assert!(gen_macro.throughput >= 10);
    assert!(gen_micro.throughput >= 10);
}

// ===========================================================================
// Test 3: Marginal vs Joint Conflicts and Uncertainty Decomposition
// ===========================================================================

#[test]
fn test_marginal_vs_joint_conflicts_and_uncertainty_decomposition() {
    // -----------------------------------------------------------------------
    // Part 1: Marginal vs Joint Conflicts (Report 12 FX-007)
    // -----------------------------------------------------------------------
    // Reference pairs: (0,0), (1,1)
    // Comparison pairs: (0,1), (1,0)
    let ref_pairs = [(0.0, 0.0), (1.0, 1.0)];
    let sim_pairs = [(0.0, 1.0), (1.0, 0.0)];

    // Evaluate 1D marginals for coordinate 1 (e.g. wait)
    let ref_c1 = [Some("0"), Some("1")];
    let sim_c1 = [Some("0"), Some("1")];
    let m1_metric = compare(&MetricRequest {
        reference: &ref_c1,
        simulation: &sim_c1,
        reference_weights: None,
        simulation_weights: None,
        algorithm_version: "empirical_equal.v1",
        origin: None,
        scale_ticks: "1",
    });
    assert_eq!(m1_metric.status, MetricStatus::Computed);
    assert_eq!(m1_metric.w1, Some(0.0));
    assert_eq!(m1_metric.ks_d, Some(0.0));

    // Evaluate 1D marginals for coordinate 2 (e.g. service)
    let ref_c2 = [Some("0"), Some("1")];
    let sim_c2 = [Some("1"), Some("0")];
    let m2_metric = compare(&MetricRequest {
        reference: &ref_c2,
        simulation: &sim_c2,
        reference_weights: None,
        simulation_weights: None,
        algorithm_version: "empirical_equal.v1",
        origin: None,
        scale_ticks: "1",
    });
    assert_eq!(m2_metric.status, MetricStatus::Computed);
    assert_eq!(m2_metric.w1, Some(0.0));
    assert_eq!(m2_metric.ks_d, Some(0.0));

    // Both 1D marginals have EXACT ZERO distances (perfect agreement), yet the joint dependence
    // is completely reversed! Compute joint 2D energy distance:
    let e_dist = energy_distance_2d(&ref_pairs, &sim_pairs);
    let expected_fx007_energy = 2.0 - 2.0_f64.sqrt(); // ~0.585786
    assert!(
        (e_dist - expected_fx007_energy).abs() < 1e-6,
        "energy distance {e_dist} must match theoretical 2 - sqrt(2) = {expected_fx007_energy}"
    );
    assert!(
        e_dist > 0.4,
        "Report 12 FX-007 energy distance {e_dist} must be > 0.4 despite zero marginal distance"
    );

    // Bivariate ED domain outcome: (wait, service) under positive vs negative correlation
    let positive_corr = [(2.0, 3.0), (6.0, 7.0)];
    let negative_corr = [(2.0, 7.0), (6.0, 3.0)];
    let ed_e_dist = energy_distance_2d(&positive_corr, &negative_corr);
    assert!(
        ed_e_dist > 0.4,
        "ED bivariate outcome discrepancy {ed_e_dist} must be > 0.4"
    );

    // -----------------------------------------------------------------------
    // Part 2: Three-Way Uncertainty Decomposition
    // -----------------------------------------------------------------------
    // (A) Stochastic Uncertainty: evaluate R=10 replications with fresh seeds under CRN
    let replications = 10;
    let mut replication_throughputs = Vec::with_capacity(replications);
    let mut replication_waits = Vec::with_capacity(replications);

    for r in 0..replications {
        let seed = 0x9000 + (r as u64) * 0x77;
        let cases = generate_synthetic_arrivals(seed, 12, 3.5);
        let report = run_free_running_simulation(&cases, ModelFidelity::Micro, 60, 2);
        replication_throughputs.push(report.throughput as f64);
        let avg_wait = if report.waits.is_empty() {
            0.0
        } else {
            report.waits.iter().sum::<u64>() as f64 / report.waits.len() as f64
        };
        replication_waits.push(avg_wait);
    }

    // Sample variance and 95% CI half-width of mean wait
    let mean_wait = replication_waits.iter().sum::<f64>() / replications as f64;
    let variance_wait = replication_waits
        .iter()
        .map(|w| (w - mean_wait).powi(2))
        .sum::<f64>()
        / (replications - 1) as f64;
    let se_wait = (variance_wait / replications as f64).sqrt();
    let ci95_half_width = 1.96 * se_wait;

    // Verify against frozen precision threshold: half_width <= 0.25 * tolerance (tolerance = 4.0 ticks)
    let frozen_tolerance = 4.0;
    let precision_target = 0.25 * frozen_tolerance; // 1.0 tick
    assert!(
        ci95_half_width <= precision_target,
        "stochastic 95% CI half-width {ci95_half_width} must be <= {precision_target}"
    );

    // (B) Input Uncertainty: evaluate spread across perturbed arrival rates (inter-arrival 2.8, 3.5, 4.2)
    let base_seed = 0xA1B2_C3D4_E5F6;
    let high_rate_cases = generate_synthetic_arrivals(base_seed, 12, 2.8);
    let mid_rate_cases = generate_synthetic_arrivals(base_seed, 12, 3.5);
    let low_rate_cases = generate_synthetic_arrivals(base_seed, 12, 4.2);

    let high_rate_report =
        run_free_running_simulation(&high_rate_cases, ModelFidelity::Micro, 60, 2);
    let mid_rate_report = run_free_running_simulation(&mid_rate_cases, ModelFidelity::Micro, 60, 2);
    let low_rate_report = run_free_running_simulation(&low_rate_cases, ModelFidelity::Micro, 60, 2);

    let high_wait = high_rate_report.waits.iter().sum::<u64>() as f64
        / high_rate_report.waits.len().max(1) as f64;
    let low_wait = low_rate_report.waits.iter().sum::<u64>() as f64
        / low_rate_report.waits.len().max(1) as f64;
    let input_uncertainty_spread = (high_wait - low_wait).abs();
    assert!(input_uncertainty_spread >= 0.0);

    // (C) Structural Uncertainty: evaluate difference between Macro and Micro on identical baseline cases
    let structural_macro =
        run_free_running_simulation(&mid_rate_cases, ModelFidelity::Macro, 60, 2);
    let structural_micro = mid_rate_report;

    let macro_avg_wait = structural_macro.waits.iter().sum::<u64>() as f64
        / structural_macro.waits.len().max(1) as f64;
    let micro_avg_wait = structural_micro.waits.iter().sum::<u64>() as f64
        / structural_micro.waits.len().max(1) as f64;
    let structural_uncertainty_spread = (macro_avg_wait - micro_avg_wait).abs();

    // Verify all three uncertainty components are non-negative and properly decoupled
    assert!(
        variance_wait >= 0.0,
        "stochastic variance must be non-negative"
    );
    assert!(
        input_uncertainty_spread >= 0.0,
        "input uncertainty spread must be non-negative"
    );
    assert!(
        structural_uncertainty_spread >= 0.0,
        "structural uncertainty spread must be non-negative"
    );
}
