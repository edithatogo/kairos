use std::ffi::{c_char, c_void};
use std::sync::atomic::{AtomicI32, Ordering};

static STEP_COUNT_FMI3: AtomicI32 = AtomicI32::new(0);
static TERMINATION_COUNT: AtomicI32 = AtomicI32::new(0);
static FREE_COUNT: AtomicI32 = AtomicI32::new(0);

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
    1
}

#[no_mangle]
pub unsafe extern "C" fn fmi3DoStep(
    _instance: *mut c_void,
    current_communication_point: f64,
    communication_step_size: f64,
    _no_set_fmustate_prior_to_current_point: bool,
    _event_handling_needed: *mut bool,
    _terminate_simulation: *mut bool,
    _early_return: *mut bool,
    last_successful_time: *mut f64,
) -> i32 {
    *last_successful_time = if current_communication_point == 10.0 {
        current_communication_point + communication_step_size / 2.0
    } else if current_communication_point == 11.0 {
        current_communication_point
    } else {
        current_communication_point + communication_step_size
    };
    STEP_COUNT_FMI3.fetch_add(1, Ordering::SeqCst);
    if current_communication_point == 12.0 {
        3
    } else if current_communication_point == 13.0 {
        4
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn fmi3Terminate(_instance: *mut c_void) -> i32 {
    TERMINATION_COUNT.fetch_add(1, Ordering::SeqCst);
    1
}

#[no_mangle]
pub unsafe extern "C" fn fmi3FreeInstance(_instance: *mut c_void) {
    FREE_COUNT.fetch_add(1, Ordering::SeqCst);
}

#[no_mangle]
pub extern "C" fn mockTerminateCount() -> i32 {
    TERMINATION_COUNT.load(Ordering::SeqCst)
}

#[no_mangle]
pub extern "C" fn mockFreeCount() -> i32 {
    FREE_COUNT.load(Ordering::SeqCst)
}

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
