use std::collections::VecDeque;
use std::num::NonZeroUsize;
use std::ops::RangeInclusive;
use std::sync::Arc;

use kairo_ecs_core::{ScheduledEventPreview, Scheduler, SchedulerCheckpoint};
use kairo_ecs_types::{DispatchedEvent, SimTime, Versioned};
use thiserror::Error;

#[derive(Clone, Debug, Eq, PartialEq, Error)]
pub enum ReplayError {
    #[error("event position {position} is outside retained positions {oldest}..={newest}")]
    PositionUnavailable {
        position: usize,
        oldest: usize,
        newest: usize,
    },
    #[error("scheduler diverged from the recorded event at position {0}")]
    Diverged(usize),
    #[error("the last step was interrupted. Restore a retained position before dispatching")]
    Interrupted,
}

struct Checkpoint<M> {
    position: usize,
    event: Option<DispatchedEvent>,
    scheduler: Versioned<SchedulerCheckpoint>,
    model: Arc<M>,
}

/// In-process debugging of a deterministic model on the Rust scheduler.
///
/// Positions count dispatched events after the starting state. A position
/// identifies the state after applying that event, including events that share
/// a simulation tick. Checkpoints cover the scheduler and caller-owned model
/// state, which must include any RNG or other state used by the transition.
///
/// The transition must be deterministic and confine its effects to the supplied
/// model and scheduler. Cloning the model must give independent mutable state.
/// Immutable data and copy-on-write pages can be shared between clones.
pub struct ReplaySession<M> {
    scheduler: Scheduler,
    model: Arc<M>,
    transition: fn(&mut M, &mut Scheduler, DispatchedEvent),
    position: usize,
    interrupted: bool,
    interval: NonZeroUsize,
    checkpoint_limit: NonZeroUsize,
    checkpoints: VecDeque<Checkpoint<M>>,
    history: VecDeque<DispatchedEvent>,
}

impl<M: Clone> ReplaySession<M> {
    /// Retain at most `checkpoint_limit` checkpoints at `interval` event
    /// boundaries, plus events after the oldest checkpoint. Set the interval
    /// and limit for the model's queue size, rewind range and restore cost.
    pub fn new(
        scheduler: Scheduler,
        model: M,
        transition: fn(&mut M, &mut Scheduler, DispatchedEvent),
        interval: NonZeroUsize,
        checkpoint_limit: NonZeroUsize,
    ) -> Self {
        let mut session = Self {
            scheduler,
            model: Arc::new(model),
            transition,
            position: 0,
            interrupted: false,
            interval,
            checkpoint_limit,
            checkpoints: VecDeque::new(),
            history: VecDeque::new(),
        };
        session.capture(None);
        session
    }

    pub fn position(&self) -> usize {
        self.position
    }

    pub fn retained_positions(&self) -> RangeInclusive<usize> {
        let oldest = self.checkpoints[0].position;
        oldest..=oldest + self.history.len()
    }

    /// # Panics
    ///
    /// Panics after an interrupted step until [`Self::seek`] restores the state.
    pub fn model(&self) -> &M {
        assert!(
            !self.interrupted,
            "restore a retained position after interruption"
        );
        &self.model
    }

    /// # Panics
    ///
    /// Panics after an interrupted step until [`Self::seek`] restores the state.
    pub fn scheduler(&self) -> &Scheduler {
        assert!(
            !self.interrupted,
            "restore a retained position after interruption"
        );
        &self.scheduler
    }

    pub fn current_event(&self) -> Option<DispatchedEvent> {
        let oldest = &self.checkpoints[0];
        if self.position == oldest.position {
            oldest.event
        } else {
            Some(self.history[self.position - oldest.position - 1])
        }
    }

    /// Dispatch the next event and apply its model transition. When stepping
    /// through retained history, verify the next event before dispatch.
    ///
    /// # Errors
    ///
    /// Returns `Diverged` if the restored scheduler would dispatch a different
    /// event from the retained history. Returns `Interrupted` if a previous
    /// step panicked and the caller has yet to restore a retained position.
    pub fn step(&mut self) -> Result<Option<DispatchedEvent>, ReplayError> {
        if self.interrupted {
            return Err(ReplayError::Interrupted);
        }
        let offset = self.position - self.checkpoints[0].position;
        let expected = self.history.get(offset);
        Self::verify_next(&mut self.scheduler, expected, self.position + 1)?;
        self.interrupted = true;
        let Some(event) = self.scheduler.step().into_dispatched() else {
            self.interrupted = false;
            return Ok(None);
        };
        (self.transition)(Arc::make_mut(&mut self.model), &mut self.scheduler, event);
        self.position += 1;
        if expected.is_none() {
            self.history.push_back(event);
            if self.position
                - self
                    .checkpoints
                    .back()
                    .expect("initial checkpoint")
                    .position
                == self.interval.get()
            {
                self.capture(Some(event));
            }
        }
        self.interrupted = false;
        Ok(Some(event))
    }

    /// Restore the nearest retained checkpoint and execute its suffix to the
    /// requested position. The current state is replaced after successful replay.
    ///
    /// # Errors
    ///
    /// Returns `PositionUnavailable` outside the retained range and `Diverged`
    /// if a scheduler event differs during replay.
    pub fn seek(&mut self, position: usize) -> Result<(), ReplayError> {
        let retained = self.retained_positions();
        if !retained.contains(&position) {
            return Err(ReplayError::PositionUnavailable {
                position,
                oldest: *retained.start(),
                newest: *retained.end(),
            });
        }
        if position == self.position && !self.interrupted {
            return Ok(());
        }
        let checkpoint = self
            .checkpoints
            .iter()
            .rfind(|checkpoint| checkpoint.position <= position)
            .expect("the oldest retained checkpoint precedes the requested position");
        let mut scheduler = Self::restore_scheduler(&checkpoint.scheduler);
        let mut model = Arc::clone(&checkpoint.model);
        let oldest = *retained.start();
        for index in checkpoint.position..position {
            Self::verify_next(
                &mut scheduler,
                Some(&self.history[index - oldest]),
                index + 1,
            )?;
            let event = scheduler
                .step()
                .into_dispatched()
                .expect("the next live event was verified");
            (self.transition)(Arc::make_mut(&mut model), &mut scheduler, event);
        }
        self.scheduler = scheduler;
        let previous_model = std::mem::replace(&mut self.model, model);
        self.position = position;
        self.interrupted = false;
        drop(previous_model);
        Ok(())
    }

    /// Seek to the first retained event dispatched at or after `tick`.
    /// Returns `None` and leaves the cursor in place when no event matches.
    ///
    /// # Errors
    ///
    /// Returns `Diverged` if a scheduler event differs during replay.
    pub fn seek_tick(&mut self, tick: SimTime) -> Result<Option<DispatchedEvent>, ReplayError> {
        let oldest = &self.checkpoints[0];
        let position = if oldest.event.is_some_and(|event| event.at >= tick) {
            oldest.position
        } else if let Some(offset) = self.history.iter().position(|event| event.at >= tick) {
            oldest.position + offset + 1
        } else {
            return Ok(None);
        };
        self.seek(position)?;
        Ok(self.current_event())
    }

    /// Create an independent timeline at the current event boundary. The
    /// edit is applied to the branch before capturing its first checkpoint.
    ///
    /// # Panics
    ///
    /// Panics after an interrupted step until [`Self::seek`] restores the state.
    pub fn fork(&self, edit: impl FnOnce(&mut M, &mut Scheduler)) -> Self {
        let mut model = self.model().clone();
        let mut scheduler = Self::restore_scheduler(&self.scheduler.checkpoint());
        edit(&mut model, &mut scheduler);
        let mut branch = Self {
            scheduler,
            model: Arc::new(model),
            transition: self.transition,
            position: self.position,
            interrupted: false,
            interval: self.interval,
            checkpoint_limit: self.checkpoint_limit,
            checkpoints: VecDeque::new(),
            history: VecDeque::new(),
        };
        branch.capture(self.current_event());
        branch
    }

    fn capture(&mut self, event: Option<DispatchedEvent>) {
        self.checkpoints.push_back(Checkpoint {
            position: self.position,
            event,
            scheduler: self.scheduler.checkpoint(),
            model: Arc::clone(&self.model),
        });
        if self.checkpoints.len() > self.checkpoint_limit.get() {
            let previous = self.checkpoints.pop_front().expect("checkpoint limit");
            self.history
                .drain(..self.checkpoints[0].position - previous.position);
            drop(previous);
        }
    }

    fn restore_scheduler(checkpoint: &Versioned<SchedulerCheckpoint>) -> Scheduler {
        Scheduler::from_checkpoint(checkpoint).expect("checkpoint created by this scheduler")
    }

    fn verify_next(
        scheduler: &mut Scheduler,
        expected: Option<&DispatchedEvent>,
        position: usize,
    ) -> Result<(), ReplayError> {
        let Some(event) = expected else {
            return Ok(());
        };
        let expected = ScheduledEventPreview {
            id: event.id,
            at: event.at,
            priority: event.priority,
            sequence: event.sequence,
            entity: event.entity,
            kind: event.kind,
        };
        if scheduler.peek_next() == Some(expected) {
            Ok(())
        } else {
            Err(ReplayError::Diverged(position))
        }
    }
}
