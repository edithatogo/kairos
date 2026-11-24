//! Native FMI 2.0 Co-Simulation counter driven by the Kairos scheduler.
//!
//! Importers must serialize calls for an instance and pass a live handle from
//! `fmi2Instantiate`. C strings must be NUL-terminated. Array arguments must
//! describe aligned buffers of the stated length. The writable output buffer
//! must be disjoint from the instance and the input buffer.

#![deny(unsafe_op_in_unsafe_fn)]

use std::ffi::{c_char, c_int, c_void, CStr};
use std::{ptr, slice};

use kairo_ecs_core::Scheduler;
use kairo_ecs_fmi::import::fmi2::Fmi2Status;
use kairo_ecs_types::{EventKind, ScheduleRequest, SimTime, StepOutcome};

pub const MODEL_GUID: &str = "{4b25542a-0db9-486b-9324-c77d4326621d}";
pub const DIGITS: u32 = 1;
pub const STEPS: u32 = 2;

const OK: c_int = Fmi2Status::Ok as c_int;
const ERROR: c_int = Fmi2Status::Error as c_int;

#[derive(Eq, PartialEq)]
enum Phase {
    Instantiated,
    Configured,
    Initializing,
    Stepping,
    Terminated,
}

struct State {
    queue: Scheduler,
    phase: Phase,
    time: f64,
    stop: Option<f64>,
    digits: u32,
}

impl State {
    fn new() -> Self {
        let mut queue = Scheduler::new();
        for (tick, priority, digit) in [(1, 3, 7), (2, 0, 2), (1, 0, 4), (3, 0, 9)] {
            queue.schedule(ScheduleRequest {
                at: SimTime::from_ticks(tick),
                priority,
                entity: None,
                kind: EventKind::custom(digit),
            });
        }
        Self {
            queue,
            phase: Phase::Instantiated,
            time: 0.0,
            stop: None,
            digits: 0,
        }
    }

    fn advance(&mut self, point: f64, step: f64) -> c_int {
        let end = point + step;
        if self.phase != Phase::Stepping
            || point != self.time
            || !end.is_finite()
            || end <= point
            || self.stop.is_some_and(|stop| end > stop)
        {
            return ERROR;
        }
        while self
            .queue
            .peek_next()
            .is_some_and(|next| next.at.ticks() as f64 <= end)
        {
            let StepOutcome::Dispatched(event) = self.queue.step() else {
                return ERROR;
            };
            self.digits = self.digits * 10 + event.kind.code();
        }
        self.time = end;
        OK
    }

    fn transition(&mut self, from: Phase, to: Phase) -> c_int {
        if self.phase != from {
            return ERROR;
        }
        self.phase = to;
        OK
    }
}

#[unsafe(no_mangle)]
extern "C" fn fmi2GetTypesPlatform() -> *const c_char {
    c"default".as_ptr()
}

#[unsafe(no_mangle)]
extern "C" fn fmi2GetVersion() -> *const c_char {
    c"2.0".as_ptr()
}

#[unsafe(no_mangle)]
unsafe extern "C" fn fmi2Instantiate(
    name: *const c_char,
    kind: c_int,
    guid: *const c_char,
    _: *const c_char,
    _: *const c_void,
    _: c_int,
    _: c_int,
) -> *mut State {
    const CO_SIMULATION: c_int = 1;
    if name.is_null()
        || guid.is_null()
        || kind != CO_SIMULATION
        || unsafe { CStr::from_ptr(guid) }.to_bytes() != MODEL_GUID.as_bytes()
    {
        return ptr::null_mut();
    }
    Box::into_raw(Box::new(State::new()))
}

#[unsafe(no_mangle)]
unsafe extern "C" fn fmi2FreeInstance(state: *mut State) {
    if !state.is_null() {
        drop(unsafe { Box::from_raw(state) });
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn fmi2SetupExperiment(
    state: *mut State,
    _: c_int,
    _: f64,
    start: f64,
    stop_defined: c_int,
    stop: f64,
) -> c_int {
    let Some(state) = (unsafe { state.as_mut() }) else {
        return ERROR;
    };
    if !matches!(state.phase, Phase::Instantiated | Phase::Configured)
        || start != 0.0
        || stop_defined != 0 && (!stop.is_finite() || stop < start)
    {
        return ERROR;
    }
    state.stop = (stop_defined != 0).then_some(stop);
    state.phase = Phase::Configured;
    OK
}

#[unsafe(no_mangle)]
unsafe extern "C" fn fmi2EnterInitializationMode(state: *mut State) -> c_int {
    unsafe { state.as_mut() }.map_or(ERROR, |state| {
        state.transition(Phase::Configured, Phase::Initializing)
    })
}

#[unsafe(no_mangle)]
unsafe extern "C" fn fmi2ExitInitializationMode(state: *mut State) -> c_int {
    unsafe { state.as_mut() }.map_or(ERROR, |state| {
        state.transition(Phase::Initializing, Phase::Stepping)
    })
}

#[unsafe(no_mangle)]
unsafe extern "C" fn fmi2DoStep(state: *mut State, point: f64, step: f64, _: c_int) -> c_int {
    unsafe { state.as_mut() }.map_or(ERROR, |state| state.advance(point, step))
}

#[unsafe(no_mangle)]
unsafe extern "C" fn fmi2GetReal(
    state: *mut State,
    refs: *const u32,
    count: usize,
    values: *mut f64,
) -> c_int {
    let Some(state) = (unsafe { state.as_ref() }) else {
        return ERROR;
    };
    if count == 0 {
        return OK;
    }
    if matches!(state.phase, Phase::Instantiated | Phase::Configured)
        || refs.is_null()
        || values.is_null()
        || !refs.is_aligned()
        || !values.is_aligned()
        || count > isize::MAX as usize / size_of::<f64>()
    {
        return ERROR;
    }
    let refs = unsafe { slice::from_raw_parts(refs, count) };
    if refs.iter().any(|value| !matches!(*value, DIGITS | STEPS)) {
        return ERROR;
    }
    let digits = f64::from(state.digits);
    let steps = state.queue.stats().dispatched_events as f64;
    for (offset, reference) in refs.iter().enumerate() {
        // The importer supplies writable storage that may be uninitialized.
        unsafe {
            values
                .add(offset)
                .write(if *reference == DIGITS { digits } else { steps })
        };
    }
    OK
}

#[unsafe(no_mangle)]
unsafe extern "C" fn fmi2Terminate(state: *mut State) -> c_int {
    unsafe { state.as_mut() }.map_or(ERROR, |state| {
        state.transition(Phase::Stepping, Phase::Terminated)
    })
}

#[unsafe(no_mangle)]
unsafe extern "C" fn fmi2Reset(state: *mut State) -> c_int {
    let Some(state) = (unsafe { state.as_mut() }) else {
        return ERROR;
    };
    *state = State::new();
    OK
}

#[unsafe(no_mangle)]
extern "C" fn fmi2SetDebugLogging(
    state: *mut State,
    _: c_int,
    categories: usize,
    _: *const *const c_char,
) -> c_int {
    if !state.is_null() && categories == 0 {
        OK
    } else {
        ERROR
    }
}

macro_rules! unused_access {
    ($($name:ident: $buffer:ty),* $(,)?) => {
        $(
            #[unsafe(no_mangle)]
            extern "C" fn $name(state: *mut State, _: *const u32, count: usize, _: $buffer) -> c_int {
                if !state.is_null() && count == 0 { OK } else { ERROR }
            }
        )*
    };
}

unused_access! {
    fmi2GetInteger: *mut c_int,
    fmi2GetBoolean: *mut c_int,
    fmi2GetString: *mut *const c_char,
    fmi2SetReal: *const f64,
    fmi2SetInteger: *const c_int,
    fmi2SetBoolean: *const c_int,
    fmi2SetString: *const *const c_char,
}

macro_rules! unsupported {
    ($($name:ident($($arg:ty),*)),* $(,)?) => {
        $(
            #[unsafe(no_mangle)]
            extern "C" fn $name(_: *mut State, $(_: $arg),*) -> c_int { ERROR }
        )*
    };
}

unsupported! {
    fmi2CancelStep(),
    fmi2SetRealInputDerivatives(*const u32, usize, *const c_int, *const f64),
    fmi2GetRealOutputDerivatives(*const u32, usize, *const c_int, *mut f64),
    fmi2GetStatus(c_int, *mut c_int),
    fmi2GetRealStatus(c_int, *mut f64),
    fmi2GetIntegerStatus(c_int, *mut c_int),
    fmi2GetBooleanStatus(c_int, *mut c_int),
    fmi2GetStringStatus(c_int, *mut *const c_char),
    fmi2GetFMUstate(*mut *mut c_void),
    fmi2SetFMUstate(*mut c_void),
    fmi2FreeFMUstate(*mut *mut c_void),
    fmi2SerializedFMUstateSize(*mut c_void, *mut usize),
    fmi2SerializeFMUstate(*mut c_void, *mut u8, usize),
    fmi2DeSerializeFMUstate(*const u8, usize, *mut *mut c_void),
    fmi2GetDirectionalDerivative(*const u32, usize, *const u32, usize, *const f64, *mut f64),
}
