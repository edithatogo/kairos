use std::ffi::{c_char, c_void};
use std::sync::atomic::{AtomicI32, Ordering};

static STEP_COUNT_FMI3: AtomicI32 = AtomicI32::new(0);

#[no_mangle]
pub unsafe extern "C" fn fmi3InstantiateCoSimulation(
    _instance_name: *const c_char,
    _instantiation_token: *const c_char,
    _resource_path: *const c_char,
    _visible: bool,
    _logging_on: bool,
    _event_mode_used: bool,
    _early_return_allowed: bool,
    _required_intermediate_variables: *const u32,
    _n_required_intermediate_variables: usize,
    _instance_environment: *const c_void,
    _log_message: *const c_void,
    _intermediate_update: *const c_void,
) -> *mut c_void {
    2 as *mut c_void
}

#[no_mangle]
pub unsafe extern "C" fn fmi3EnterInitializationMode(
    _instance: *mut c_void,
    _tolerance_defined: bool,
    _tolerance: f64,
    _start_time: f64,
    _stop_time_defined: bool,
    _stop_time: f64,
) -> i32 {
    0
}

#[no_mangle]
pub unsafe extern "C" fn fmi3ExitInitializationMode(_instance: *mut c_void) -> i32 {
    0
}

#[no_mangle]
pub unsafe extern "C" fn fmi3DoStep(
    _instance: *mut c_void,
    _current_communication_point: f64,
    _communication_step_size: f64,
    _no_set_fmustate_prior_to_current_point: bool,
    _event_handling_needed: *mut bool,
    _terminate_simulation: *mut bool,
    _early_return_time: *mut f64,
) -> i32 {
    STEP_COUNT_FMI3.fetch_add(1, Ordering::SeqCst);
    0
}

#[no_mangle]
pub unsafe extern "C" fn fmi3Terminate(_instance: *mut c_void) -> i32 {
    0
}

#[no_mangle]
pub unsafe extern "C" fn fmi3FreeInstance(_instance: *mut c_void) {}

#[no_mangle]
pub unsafe extern "C" fn fmi3GetFloat64(
    _instance: *mut c_void,
    _value_references: *const u32,
    n_value_references: usize,
    values: *mut f64,
    _n_values: usize,
) -> i32 {
    for i in 0..n_value_references {
        *values.add(i) = 100.0 + (STEP_COUNT_FMI3.load(Ordering::SeqCst) as f64);
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn fmi3SetFloat64(
    _instance: *mut c_void,
    _value_references: *const u32,
    _n_value_references: usize,
    _values: *const f64,
    _n_values: usize,
) -> i32 {
    0
}
