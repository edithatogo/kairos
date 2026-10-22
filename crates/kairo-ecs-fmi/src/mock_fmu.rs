use std::ffi::{c_char, c_void};
use std::sync::atomic::{AtomicI32, Ordering};

static STEP_COUNT: AtomicI32 = AtomicI32::new(0);

#[no_mangle]
pub unsafe extern "C" fn fmi2Instantiate(
    _instance_name: *const c_char,
    _fmu_type: i32,
    _fmu_guid: *const c_char,
    _fmu_resource_location: *const c_char,
    _functions: *const c_void,
    _visible: i32,
    _logging_on: i32,
) -> *mut c_void {
    // Return dummy non-null pointer
    1 as *mut c_void
}

#[no_mangle]
pub unsafe extern "C" fn fmi2SetupExperiment(
    _c: *mut c_void,
    _tolerance_defined: i32,
    _tolerance: f64,
    _start_time: f64,
    _stop_time_defined: i32,
    _stop_time: f64,
) -> i32 {
    0 // fmi2OK
}

#[no_mangle]
pub unsafe extern "C" fn fmi2EnterInitializationMode(_c: *mut c_void) -> i32 {
    0
}

#[no_mangle]
pub unsafe extern "C" fn fmi2ExitInitializationMode(_c: *mut c_void) -> i32 {
    0
}

#[no_mangle]
pub unsafe extern "C" fn fmi2DoStep(
    _c: *mut c_void,
    _current_communication_point: f64,
    _communication_step_size: f64,
    _no_set_fmustate_prior_to_current_point: i32,
) -> i32 {
    STEP_COUNT.fetch_add(1, Ordering::SeqCst);
    0
}

#[no_mangle]
pub unsafe extern "C" fn fmi2Terminate(_c: *mut c_void) -> i32 {
    0
}

#[no_mangle]
pub unsafe extern "C" fn fmi2FreeInstance(_c: *mut c_void) {}

#[no_mangle]
pub unsafe extern "C" fn fmi2GetReal(
    _c: *mut c_void,
    _vr: *const u32,
    nvr: usize,
    value: *mut f64,
) -> i32 {
    for i in 0..nvr {
        *value.add(i) = 42.0 + (STEP_COUNT.load(Ordering::SeqCst) as f64);
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn fmi2SetReal(
    _c: *mut c_void,
    _vr: *const u32,
    _nvr: usize,
    _value: *const f64,
) -> i32 {
    0
}

#[no_mangle]
pub unsafe extern "C" fn fmi2GetInteger(
    _c: *mut c_void,
    _vr: *const u32,
    nvr: usize,
    value: *mut i32,
) -> i32 {
    for i in 0..nvr {
        *value.add(i) = STEP_COUNT.load(Ordering::SeqCst);
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn fmi2SetInteger(
    _c: *mut c_void,
    _vr: *const u32,
    _nvr: usize,
    _value: *const i32,
) -> i32 {
    0
}

#[no_mangle]
pub unsafe extern "C" fn fmi2GetBoolean(
    _c: *mut c_void,
    _vr: *const u32,
    nvr: usize,
    value: *mut i32,
) -> i32 {
    for i in 0..nvr {
        *value.add(i) = 1;
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn fmi2SetBoolean(
    _c: *mut c_void,
    _vr: *const u32,
    _nvr: usize,
    _value: *const i32,
) -> i32 {
    0
}

#[no_mangle]
pub unsafe extern "C" fn fmi2GetString(
    _c: *mut c_void,
    _vr: *const u32,
    _nvr: usize,
    _value: *mut *const c_char,
) -> i32 {
    0
}

#[no_mangle]
pub unsafe extern "C" fn fmi2SetString(
    _c: *mut c_void,
    _vr: *const u32,
    _nvr: usize,
    _value: *const *const c_char,
) -> i32 {
    0
}
