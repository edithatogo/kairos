//! FMI 3.0 ABI for the same scheduled counter used by FMI 2.0.

use std::ffi::{c_char, c_int, c_void, CStr};
use std::ptr;

use super::{read_reals, Phase, State, ERROR, MODEL_GUID, OK};

type LogMessage = Option<unsafe extern "C" fn(*mut c_void, c_int, *const c_char, *const c_char)>;
type IntermediateUpdate =
    Option<unsafe extern "C" fn(*mut c_void, f64, bool, bool, bool, bool, *mut bool, *mut f64)>;
type ClockUpdate = Option<unsafe extern "C" fn(*mut c_void)>;
type Preemption = Option<unsafe extern "C" fn()>;

struct Counter {
    model: State,
    environment: *mut c_void,
    log: LogMessage,
    logging: bool,
}

impl Counter {
    fn report(&self, status: c_int, message: &CStr) -> c_int {
        if status != OK && self.logging {
            if let Some(log) = self.log {
                unsafe { log(self.environment, status, c"".as_ptr(), message.as_ptr()) };
            }
        }
        status
    }
}

#[unsafe(no_mangle)]
extern "C" fn fmi3GetVersion() -> *const c_char {
    c"3.0".as_ptr()
}

#[unsafe(no_mangle)]
unsafe extern "C" fn fmi3InstantiateCoSimulation(
    name: *const c_char,
    token: *const c_char,
    _: *const c_char,
    _: bool,
    logging: bool,
    event_mode: bool,
    _: bool,
    _: *const u32,
    intermediate_count: usize,
    environment: *mut c_void,
    log: LogMessage,
    _: IntermediateUpdate,
) -> *mut Counter {
    if name.is_null()
        || token.is_null()
        || event_mode
        || intermediate_count != 0
        || unsafe { CStr::from_ptr(token) }.to_bytes() != MODEL_GUID.as_bytes()
        || unsafe { CStr::from_ptr(name) }
            .to_bytes()
            .iter()
            .all(u8::is_ascii_whitespace)
    {
        if let Some(log) = log.filter(|_| logging) {
            unsafe {
                log(
                    environment,
                    ERROR,
                    c"".as_ptr(),
                    c"invalid counter instantiation parameters".as_ptr(),
                )
            };
        }
        return ptr::null_mut();
    }
    Box::into_raw(Box::new(Counter {
        model: State::new(),
        environment,
        log,
        logging,
    }))
}

#[unsafe(no_mangle)]
unsafe extern "C" fn fmi3FreeInstance(instance: *mut Counter) {
    if !instance.is_null() {
        drop(unsafe { Box::from_raw(instance) });
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn fmi3EnterInitializationMode(
    instance: *mut Counter,
    _: bool,
    _: f64,
    start: f64,
    stop_defined: bool,
    stop: f64,
) -> c_int {
    let Some(instance) = (unsafe { instance.as_mut() }) else {
        return ERROR;
    };
    let status = instance.model.configure(start, stop_defined, stop);
    if status != OK {
        return instance.report(
            status,
            c"counter must start at time zero from the instantiated state",
        );
    }
    instance.model.phase = Phase::Initializing;
    OK
}

#[unsafe(no_mangle)]
unsafe extern "C" fn fmi3ExitInitializationMode(instance: *mut Counter) -> c_int {
    let Some(instance) = (unsafe { instance.as_mut() }) else {
        return ERROR;
    };
    let status = instance
        .model
        .transition(Phase::Initializing, Phase::Stepping);
    instance.report(status, c"counter is outside initialization mode")
}

#[unsafe(no_mangle)]
unsafe extern "C" fn fmi3DoStep(
    instance: *mut Counter,
    point: f64,
    step: f64,
    _: bool,
    event_handling_needed: *mut bool,
    terminate_simulation: *mut bool,
    early_return: *mut bool,
    last_successful_time: *mut f64,
) -> c_int {
    let Some(instance) = (unsafe { instance.as_mut() }) else {
        return ERROR;
    };
    if event_handling_needed.is_null()
        || terminate_simulation.is_null()
        || early_return.is_null()
        || last_successful_time.is_null()
        || !last_successful_time.is_aligned()
    {
        return instance.report(ERROR, c"invalid counter step result buffers");
    }
    let status = instance.model.advance(point, step);
    if status != OK {
        return instance.report(status, c"invalid counter communication point or step");
    }
    unsafe {
        event_handling_needed.write(false);
        terminate_simulation.write(false);
        early_return.write(false);
        last_successful_time.write(instance.model.time);
    }
    OK
}

#[unsafe(no_mangle)]
unsafe extern "C" fn fmi3GetFloat64(
    instance: *mut Counter,
    references: *const u32,
    count: usize,
    values: *mut f64,
    value_count: usize,
) -> c_int {
    let Some(instance) = (unsafe { instance.as_ref() }) else {
        return ERROR;
    };
    if count != value_count {
        return instance.report(ERROR, c"counter outputs are scalars");
    }
    let status = unsafe { read_reals(&instance.model, references, count, values, true) };
    instance.report(status, c"invalid counter output request")
}

#[unsafe(no_mangle)]
unsafe extern "C" fn fmi3Terminate(instance: *mut Counter) -> c_int {
    let Some(instance) = (unsafe { instance.as_mut() }) else {
        return ERROR;
    };
    let status = instance
        .model
        .transition(Phase::Stepping, Phase::Terminated);
    instance.report(status, c"counter is outside step mode")
}

#[unsafe(no_mangle)]
unsafe extern "C" fn fmi3Reset(instance: *mut Counter) -> c_int {
    let Some(instance) = (unsafe { instance.as_mut() }) else {
        return ERROR;
    };
    instance.model = State::new();
    OK
}

#[unsafe(no_mangle)]
unsafe extern "C" fn fmi3SetDebugLogging(
    instance: *mut Counter,
    logging: bool,
    count: usize,
    _: *const *const c_char,
) -> c_int {
    let Some(instance) = (unsafe { instance.as_mut() }) else {
        return ERROR;
    };
    if count != 0 {
        return instance.report(ERROR, c"counter has no named log categories");
    }
    instance.logging = logging;
    OK
}

// FMI 3.0 binary FMUs export all of the API's symbols. These entry points
// reject the two undeclared interfaces and optional capabilities of this model.
#[unsafe(no_mangle)]
extern "C" fn fmi3InstantiateModelExchange(
    _: *const c_char,
    _: *const c_char,
    _: *const c_char,
    _: bool,
    _: bool,
    _: *mut c_void,
    _: LogMessage,
) -> *mut Counter {
    ptr::null_mut()
}

#[unsafe(no_mangle)]
extern "C" fn fmi3InstantiateScheduledExecution(
    _: *const c_char,
    _: *const c_char,
    _: *const c_char,
    _: bool,
    _: bool,
    _: *mut c_void,
    _: LogMessage,
    _: ClockUpdate,
    _: Preemption,
    _: Preemption,
) -> *mut Counter {
    ptr::null_mut()
}

macro_rules! unused_scalars {
    ($($name:ident: $buffer:ty),* $(,)?) => {
        $(
            #[unsafe(no_mangle)]
            unsafe extern "C" fn $name(instance: *mut Counter, _: *const u32, references: usize, _: $buffer, values: usize) -> c_int {
                let Some(instance) = (unsafe { instance.as_ref() }) else { return ERROR; };
                if references == 0 && values == 0 { OK }
                else { instance.report(ERROR, c"no writable or matching scalar variables") }
            }
        )*
    };
}

unused_scalars! {
    fmi3GetFloat32: *mut f32,
    fmi3GetInt8: *mut i8,
    fmi3GetUInt8: *mut u8,
    fmi3GetInt16: *mut i16,
    fmi3GetUInt16: *mut u16,
    fmi3GetInt32: *mut i32,
    fmi3GetUInt32: *mut u32,
    fmi3GetInt64: *mut i64,
    fmi3GetUInt64: *mut u64,
    fmi3GetBoolean: *mut bool,
    fmi3GetString: *mut *const c_char,
    fmi3SetFloat32: *const f32,
    fmi3SetFloat64: *const f64,
    fmi3SetInt8: *const i8,
    fmi3SetUInt8: *const u8,
    fmi3SetInt16: *const i16,
    fmi3SetUInt16: *const u16,
    fmi3SetInt32: *const i32,
    fmi3SetUInt32: *const u32,
    fmi3SetInt64: *const i64,
    fmi3SetUInt64: *const u64,
    fmi3SetBoolean: *const bool,
    fmi3SetString: *const *const c_char,
}

macro_rules! unsupported {
    ($($name:ident($($arg:ty),*)),* $(,)?) => {
        $(
            #[unsafe(no_mangle)]
            unsafe extern "C" fn $name(instance: *mut Counter, $(_: $arg),*) -> c_int {
                unsafe { instance.as_ref() }.map_or(ERROR, |instance| {
                    instance.report(ERROR, c"counter does not provide this capability")
                })
            }
        )*
    };
}

unsupported! {
    fmi3EnterEventMode(),
    fmi3GetBinary(*const u32, usize, *mut usize, *mut *const u8, usize),
    fmi3SetBinary(*const u32, usize, *const usize, *const *const u8, usize),
    fmi3GetClock(*const u32, usize, *mut bool),
    fmi3SetClock(*const u32, usize, *const bool),
    fmi3GetNumberOfVariableDependencies(u32, *mut usize),
    fmi3GetVariableDependencies(u32, *mut usize, *mut u32, *mut usize, *mut c_int, usize),
    fmi3GetFMUState(*mut *mut c_void),
    fmi3SetFMUState(*mut c_void),
    fmi3FreeFMUState(*mut *mut c_void),
    fmi3SerializedFMUStateSize(*mut c_void, *mut usize),
    fmi3SerializeFMUState(*mut c_void, *mut u8, usize),
    fmi3DeserializeFMUState(*const u8, usize, *mut *mut c_void),
    fmi3GetDirectionalDerivative(*const u32, usize, *const u32, usize, *const f64, usize, *mut f64, usize),
    fmi3GetAdjointDerivative(*const u32, usize, *const u32, usize, *const f64, usize, *mut f64, usize),
    fmi3EnterConfigurationMode(),
    fmi3ExitConfigurationMode(),
    fmi3GetIntervalDecimal(*const u32, usize, *mut f64, *mut c_int),
    fmi3GetIntervalFraction(*const u32, usize, *mut u64, *mut u64, *mut c_int),
    fmi3GetShiftDecimal(*const u32, usize, *mut f64),
    fmi3GetShiftFraction(*const u32, usize, *mut u64, *mut u64),
    fmi3SetIntervalDecimal(*const u32, usize, *const f64),
    fmi3SetIntervalFraction(*const u32, usize, *const u64, *const u64),
    fmi3SetShiftDecimal(*const u32, usize, *const f64),
    fmi3SetShiftFraction(*const u32, usize, *const u64, *const u64),
    fmi3EvaluateDiscreteStates(),
    fmi3UpdateDiscreteStates(*mut bool, *mut bool, *mut bool, *mut bool, *mut bool, *mut f64),
    fmi3EnterContinuousTimeMode(),
    fmi3CompletedIntegratorStep(bool, *mut bool, *mut bool),
    fmi3SetTime(f64),
    fmi3SetContinuousStates(*const f64, usize),
    fmi3GetContinuousStateDerivatives(*mut f64, usize),
    fmi3GetEventIndicators(*mut f64, usize),
    fmi3GetContinuousStates(*mut f64, usize),
    fmi3GetNominalsOfContinuousStates(*mut f64, usize),
    fmi3GetNumberOfEventIndicators(*mut usize),
    fmi3GetNumberOfContinuousStates(*mut usize),
    fmi3EnterStepMode(),
    fmi3GetOutputDerivatives(*const u32, usize, *const i32, *mut f64, usize),
    fmi3ActivateModelPartition(u32, f64),
}
