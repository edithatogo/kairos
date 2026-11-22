use std::ffi::{c_char, c_void};

pub type Fmi2Component = *mut c_void;
pub type Fmi2ValueReference = u32;
pub type Fmi2Real = f64;
pub type Fmi2Integer = i32;
pub type Fmi2Boolean = i32;
pub type Fmi2String = *const c_char;

#[repr(C)]
pub(crate) struct Fmi2Callbacks {
    logger: unsafe extern "C" fn(*mut c_void, Fmi2String, i32, Fmi2String, Fmi2String, ...),
    allocate_memory: unsafe extern "C" fn(usize, usize) -> *mut c_void,
    free_memory: unsafe extern "C" fn(*mut c_void),
    step_finished: Option<unsafe extern "C" fn(*mut c_void, i32)>,
    environment: *mut c_void,
}

unsafe extern "C" {
    fn calloc(count: usize, size: usize) -> *mut c_void;
    fn free(memory: *mut c_void);
}

// Import errors are reported through FMI status codes. The optional log text
// and its C variadic arguments need no decoding on the Rust side.
unsafe extern "C" fn discard_log(
    _: *mut c_void,
    _: Fmi2String,
    _: i32,
    _: Fmi2String,
    _: Fmi2String,
    _: ...
) {
}

impl Fmi2Callbacks {
    pub(crate) fn synchronous() -> Self {
        Self {
            logger: discard_log,
            allocate_memory: calloc,
            free_memory: free,
            step_finished: None,
            environment: std::ptr::null_mut(),
        }
    }
}

#[repr(i32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Fmi2Status {
    Ok = 0,
    Warning = 1,
    Discard = 2,
    Error = 3,
    Fatal = 4,
    Pending = 5,
}

impl Fmi2Status {
    pub fn from_raw(value: i32) -> Self {
        match value {
            0 => Self::Ok,
            1 => Self::Warning,
            2 => Self::Discard,
            3 => Self::Error,
            4 => Self::Fatal,
            5 => Self::Pending,
            _ => Self::Fatal,
        }
    }

    pub fn is_success(self) -> bool {
        matches!(self, Self::Ok | Self::Warning)
    }
}

pub type Fmi2SetupExperiment = unsafe extern "C" fn(
    Fmi2Component,
    Fmi2Boolean,
    Fmi2Real,
    Fmi2Real,
    Fmi2Boolean,
    Fmi2Real,
) -> i32;
pub type Fmi2EnterInitializationMode = unsafe extern "C" fn(Fmi2Component) -> i32;
pub type Fmi2ExitInitializationMode = unsafe extern "C" fn(Fmi2Component) -> i32;
pub type Fmi2DoStep = unsafe extern "C" fn(Fmi2Component, Fmi2Real, Fmi2Real, Fmi2Boolean) -> i32;
pub type Fmi2Terminate = unsafe extern "C" fn(Fmi2Component) -> i32;
pub type Fmi2FreeInstance = unsafe extern "C" fn(Fmi2Component);
pub type Fmi2GetReal =
    unsafe extern "C" fn(Fmi2Component, *const Fmi2ValueReference, usize, *mut Fmi2Real) -> i32;
pub type Fmi2SetReal =
    unsafe extern "C" fn(Fmi2Component, *const Fmi2ValueReference, usize, *const Fmi2Real) -> i32;
pub type Fmi2GetInteger =
    unsafe extern "C" fn(Fmi2Component, *const Fmi2ValueReference, usize, *mut Fmi2Integer) -> i32;
pub type Fmi2SetInteger = unsafe extern "C" fn(
    Fmi2Component,
    *const Fmi2ValueReference,
    usize,
    *const Fmi2Integer,
) -> i32;
pub type Fmi2GetBoolean =
    unsafe extern "C" fn(Fmi2Component, *const Fmi2ValueReference, usize, *mut Fmi2Boolean) -> i32;
pub type Fmi2SetBoolean = unsafe extern "C" fn(
    Fmi2Component,
    *const Fmi2ValueReference,
    usize,
    *const Fmi2Boolean,
) -> i32;
pub type Fmi2GetString =
    unsafe extern "C" fn(Fmi2Component, *const Fmi2ValueReference, usize, *mut Fmi2String) -> i32;
pub type Fmi2SetString =
    unsafe extern "C" fn(Fmi2Component, *const Fmi2ValueReference, usize, *const Fmi2String) -> i32;

#[derive(Clone, Copy)]
pub struct Fmi2FunctionTable {
    pub setup_experiment: Fmi2SetupExperiment,
    pub enter_initialization_mode: Fmi2EnterInitializationMode,
    pub exit_initialization_mode: Fmi2ExitInitializationMode,
    pub do_step: Fmi2DoStep,
    pub terminate: Fmi2Terminate,
    pub free_instance: Fmi2FreeInstance,
    pub get_real: Fmi2GetReal,
    pub set_real: Fmi2SetReal,
    pub get_integer: Fmi2GetInteger,
    pub set_integer: Fmi2SetInteger,
    pub get_boolean: Fmi2GetBoolean,
    pub set_boolean: Fmi2SetBoolean,
    pub get_string: Fmi2GetString,
    pub set_string: Fmi2SetString,
}
