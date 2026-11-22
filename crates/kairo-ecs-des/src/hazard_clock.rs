#![forbid(unsafe_code)]

use kairo_ecs_core::Scheduler;
use kairo_ecs_rng::{
    draw_standard_threshold, CauseId, DeterministicStream, HazardRate, RealizedEvent, TieBreaker,
};
use kairo_ecs_types::{EntityId, EventId, EventKind, ScheduleRequest, SimTime};
use std::collections::HashMap;

/// Policy for managing accumulated hazard / threshold upon covariate change or process interruption.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HazardInterruptionPolicy {
    /// Sample a fresh threshold when replacing a hazard channel.
    Restart,
    /// Retain residual threshold: remaining threshold W_rem = W_total - H(t_elapsed).
    RetainResidual,
}

/// Tracking record for an active survival clock hazard channel.
#[derive(Clone, Debug)]
pub struct HazardChannelState {
    pub cause_id: CauseId,
    pub priority: u32,
    pub hazard: HazardRate,
    /// Accumulated integrated hazard already expended in prior intervals.
    pub accumulated_hazard: f64,
    /// Current standard threshold W ~ Exp(1) assigned to this channel.
    pub target_threshold: f64,
    /// Calendar time when the current interval started.
    pub interval_start_time: f64,
    /// Active age on the current hazard curve at the start of the interval.
    active_age_at_interval_start: f64,
}

impl HazardChannelState {
    fn accrue_until(&mut self, now: f64) {
        let aged = self.active_age_at_interval_start + (now - self.interval_start_time).max(0.0);
        self.accumulated_hazard += self.hazard.cumulative_hazard(aged)
            - self
                .hazard
                .cumulative_hazard(self.active_age_at_interval_start);
        self.active_age_at_interval_start = aged;
        self.interval_start_time = now;
    }
}

/// Survival clock tracking continuous progression and competing risk events for a process or entity.
/// Operates in continuous simulation time (mapped to SimTime ticks via tick_scale).
#[derive(Clone, Debug)]
pub struct SurvivalClock {
    pub process_id: u64,
    pub entity: Option<EntityId>,
    pub policy: HazardInterruptionPolicy,
    pub tie_breaker: TieBreaker,
    pub tick_scale: f64, // ticks per unit of continuous time
    pub channels: HashMap<CauseId, HazardChannelState>,
    pub scheduled_event_id: Option<EventId>,
    pub current_winner: Option<(CauseId, f64)>, // (cause_id, realization_calendar_time)
    pub is_active: bool,
    pub last_update_time: f64,
    paused: bool,
    current_is_censored: bool,
}

impl SurvivalClock {
    pub const SEMANTICS_VERSION: &'static str = "elapsed-active-age-v2";

    /// Create a new survival clock.
    /// `tick_scale`: conversion factor where `sim_time_ticks = (continuous_time * tick_scale) as u64`.
    pub fn new(
        process_id: u64,
        entity: Option<EntityId>,
        policy: HazardInterruptionPolicy,
        tie_breaker: TieBreaker,
        tick_scale: f64,
    ) -> Self {
        Self {
            process_id,
            entity,
            policy,
            tie_breaker,
            tick_scale: if tick_scale <= 0.0 { 1.0 } else { tick_scale },
            channels: HashMap::new(),
            scheduled_event_id: None,
            current_winner: None,
            is_active: false,
            last_update_time: 0.0,
            paused: false,
            current_is_censored: false,
        }
    }

    /// Add or update a cause hazard specification.
    /// Replacing the channel starts the new hazard curve at age zero.
    /// `RetainResidual` carries over the unspent threshold; `Restart` draws a fresh one.
    pub fn set_channel(
        &mut self,
        cause_id: CauseId,
        priority: u32,
        hazard: HazardRate,
        now: f64,
        stream: &mut DeterministicStream,
    ) {
        if let Some(existing) = self.channels.get_mut(&cause_id) {
            if !self.paused {
                existing.accrue_until(now);
            }
            existing.interval_start_time = now;
            existing.active_age_at_interval_start = 0.0;
            existing.hazard = hazard;
            existing.priority = priority;

            if self.policy == HazardInterruptionPolicy::Restart {
                existing.target_threshold = draw_standard_threshold(stream);
                existing.accumulated_hazard = 0.0;
            }
        } else {
            let target_w = draw_standard_threshold(stream);
            self.channels.insert(
                cause_id,
                HazardChannelState {
                    cause_id,
                    priority,
                    hazard,
                    accumulated_hazard: 0.0,
                    target_threshold: target_w,
                    interval_start_time: now,
                    active_age_at_interval_start: 0.0,
                },
            );
        }
    }

    /// Remove a competing hazard channel (e.g. cure, repair completed).
    pub fn remove_channel(&mut self, cause_id: CauseId) {
        self.channels.remove(&cause_id);
    }

    /// Evaluate the earliest competing event, reporting an overdue winner at `now`.
    pub fn compute_earliest_event(&self, now: f64) -> Option<(CauseId, f64)> {
        self.predicted_event()
            .map(|(cause, at)| (cause, at.max(now)))
    }

    fn predicted_event(&self) -> Option<(CauseId, f64)> {
        if self.paused {
            return None;
        }

        let mut candidate_events: Vec<(CauseId, u32, f64)> = Vec::new();

        // Sort causes deterministically by CauseId
        let mut causes: Vec<&HazardChannelState> = self.channels.values().collect();
        causes.sort_by_key(|c| c.cause_id);

        for channel in causes {
            let remaining_w = channel.target_threshold - channel.accumulated_hazard;
            if remaining_w <= 0.0 {
                candidate_events.push((
                    channel.cause_id,
                    channel.priority,
                    channel.interval_start_time,
                ));
                continue;
            }
            if let Some(dt) = channel
                .hazard
                .invert_cumulative_hazard(channel.active_age_at_interval_start, remaining_w)
            {
                let at = channel.interval_start_time + dt;
                if dt >= 0.0 && at.is_finite() {
                    candidate_events.push((channel.cause_id, channel.priority, at));
                }
            }
        }

        if candidate_events.is_empty() {
            return None;
        }

        // Sort candidates by scheduled event time, breaking ties per policy
        candidate_events.sort_by(|a, b| {
            let time_cmp = a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal);
            if time_cmp != std::cmp::Ordering::Equal {
                return time_cmp;
            }
            match self.tie_breaker {
                TieBreaker::LowestCauseId => a.0.cmp(&b.0),
                TieBreaker::HighestCauseId => b.0.cmp(&a.0),
                TieBreaker::ExplicitPriority => {
                    let prio_cmp = b.1.cmp(&a.1); // higher priority first
                    if prio_cmp != std::cmp::Ordering::Equal {
                        prio_cmp
                    } else {
                        a.0.cmp(&b.0)
                    }
                }
            }
        });

        let (winner_cause, _, min_time) = candidate_events[0];
        Some((winner_cause, min_time))
    }

    /// Synchronize and reschedule the survival clock in the DES scheduler.
    /// If an existing scheduled event exists and the realization time/cause changed, cancel it deterministically.
    pub fn reschedule(
        &mut self,
        scheduler: &mut Scheduler,
        now: f64,
        censor_time: Option<f64>,
        priority: i32,
    ) -> Option<RealizedEvent> {
        self.last_update_time = now;

        // Cancel previously scheduled scheduler event if pending
        if let Some(ev_id) = self.scheduled_event_id.take() {
            let _ = scheduler.cancel(ev_id);
        }
        self.current_winner = None;
        self.is_active = false;
        if self.paused {
            return None;
        }

        let earliest = self.predicted_event();

        let final_event = match (earliest, censor_time) {
            (Some((cause, t_event)), Some(t_censor)) => {
                if t_event <= t_censor {
                    RealizedEvent {
                        winning_cause: cause,
                        delta_time: (t_event - now).max(0.0),
                        is_censored: false,
                    }
                } else {
                    RealizedEvent {
                        winning_cause: CauseId(u32::MAX),
                        delta_time: (t_censor - now).max(0.0),
                        is_censored: true,
                    }
                }
            }
            (Some((cause, t_event)), None) => RealizedEvent {
                winning_cause: cause,
                delta_time: (t_event - now).max(0.0),
                is_censored: false,
            },
            (None, Some(t_censor)) => {
                if t_censor >= now {
                    RealizedEvent {
                        winning_cause: CauseId(u32::MAX),
                        delta_time: t_censor - now,
                        is_censored: true,
                    }
                } else {
                    return None;
                }
            }
            (None, None) => return None,
        };

        // Schedule event in core scheduler
        let target_ticks = ((now + final_event.delta_time) * self.tick_scale).round() as u128;
        let event_id = scheduler.schedule(ScheduleRequest {
            at: SimTime::from_ticks(target_ticks),
            priority,
            entity: self.entity,
            kind: EventKind::Custom(final_event.winning_cause.0),
        });
        self.scheduled_event_id = Some(event_id);
        self.current_winner = Some((final_event.winning_cause, now + final_event.delta_time));
        self.current_is_censored = final_event.is_censored;
        self.is_active = true;

        Some(final_event)
    }

    /// Suspend/pause clock progression (e.g. entity pauses during transport holds).
    /// Freezes cumulative hazard without discarding residual thresholds.
    pub fn pause(&mut self, scheduler: &mut Scheduler, now: f64) {
        if self.paused {
            return;
        }
        if let Some(ev_id) = self.scheduled_event_id.take() {
            let _ = scheduler.cancel(ev_id);
        }
        // Accumulate elapsed hazard up to now
        for channel in self.channels.values_mut() {
            channel.accrue_until(now);
        }
        self.current_winner = None;
        self.is_active = false;
        self.last_update_time = now;
        self.paused = true;
    }

    /// Resume clock progression after pause.
    pub fn resume(
        &mut self,
        scheduler: &mut Scheduler,
        now: f64,
        censor_time: Option<f64>,
        priority: i32,
    ) -> Option<RealizedEvent> {
        if self.paused {
            for channel in self.channels.values_mut() {
                channel.interval_start_time = now;
            }
            self.paused = false;
        }
        self.reschedule(scheduler, now, censor_time, priority)
    }

    /// Handle dispatch of the scheduled event at current simulation time.
    /// Resets the clock or marks it inactive.
    pub fn on_event_dispatched(&mut self) -> Option<RealizedEvent> {
        self.scheduled_event_id = None;
        self.is_active = false;
        if let Some((cause, realization_time)) = self.current_winner.take() {
            Some(RealizedEvent {
                winning_cause: cause,
                delta_time: (realization_time - self.last_update_time).max(0.0),
                is_censored: self.current_is_censored,
            })
        } else {
            None
        }
    }
}
