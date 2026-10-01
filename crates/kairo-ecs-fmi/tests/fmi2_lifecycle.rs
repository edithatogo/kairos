#![cfg(feature = "fmi2")]

use std::ffi::c_void;
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};

use kairo_ecs_fmi::import::fmi2::{Fmi2Component, Fmi2FunctionTable};
use kairo_ecs_fmi::import::instance::{Fmi2CoSimulationInstance, Fmi2LifecycleState, FmuInstance};
use kairo_ecs_fmi::FmiError;

static SETUP_COUNT: AtomicUsize = AtomicUsize::new(0);
static ENTER_COUNT: AtomicUsize = AtomicUsize::new(0);
static EXIT_COUNT: AtomicUsize = AtomicUsize::new(0);
static STEP_COUNT: AtomicUsize = AtomicUsize::new(0);
static TERMINATE_COUNT: AtomicUsize = AtomicUsize::new(0);
static FREE_COUNT: AtomicUsize = AtomicUsize::new(0);

unsafe extern "C" fn setup(_: Fmi2Component, _: i32, _: f64, _: f64, _: i32, _: f64) -> i32 {
    SETUP_COUNT.fetch_add(1, Ordering::SeqCst);
    0
}

unsafe extern "C" fn enter(_: Fmi2Component) -> i32 {
    ENTER_COUNT.fetch_add(1, Ordering::SeqCst);
    0
}

unsafe extern "C" fn exit(_: Fmi2Component) -> i32 {
    EXIT_COUNT.fetch_add(1, Ordering::SeqCst);
    0
}

unsafe extern "C" fn step(_: Fmi2Component, _: f64, _: f64, _: i32) -> i32 {
    STEP_COUNT.fetch_add(1, Ordering::SeqCst);
    0
}

unsafe extern "C" fn terminate(_: Fmi2Component) -> i32 {
    TERMINATE_COUNT.fetch_add(1, Ordering::SeqCst);
    0
}

unsafe extern "C" fn free(_: Fmi2Component) {
    FREE_COUNT.fetch_add(1, Ordering::SeqCst);
}

unsafe extern "C" fn get_real(_: Fmi2Component, _: *const u32, n: usize, values: *mut f64) -> i32 {
    for index in 0..n {
        *values.add(index) = index as f64;
    }
    0
}

unsafe extern "C" fn set_real(_: Fmi2Component, _: *const u32, _: usize, _: *const f64) -> i32 {
    0
}

unsafe extern "C" fn get_integer(
    _: Fmi2Component,
    _: *const u32,
    n: usize,
    values: *mut i32,
) -> i32 {
    for index in 0..n {
        *values.add(index) = index as i32;
    }
    0
}

unsafe extern "C" fn set_integer(_: Fmi2Component, _: *const u32, _: usize, _: *const i32) -> i32 {
    0
}

unsafe extern "C" fn get_boolean(
    _: Fmi2Component,
    _: *const u32,
    n: usize,
    values: *mut i32,
) -> i32 {
    for index in 0..n {
        *values.add(index) = (index % 2) as i32;
    }
    0
}

unsafe extern "C" fn set_boolean(_: Fmi2Component, _: *const u32, _: usize, _: *const i32) -> i32 {
    0
}

unsafe extern "C" fn get_string(
    _: Fmi2Component,
    _: *const u32,
    _: usize,
    _: *mut *const i8,
) -> i32 {
    0
}

unsafe extern "C" fn set_string(
    _: Fmi2Component,
    _: *const u32,
    _: usize,
    _: *const *const i8,
) -> i32 {
    0
}

fn table() -> Fmi2FunctionTable {
    Fmi2FunctionTable {
        setup_experiment: setup,
        enter_initialization_mode: enter,
        exit_initialization_mode: exit,
        do_step: step,
        terminate,
        free_instance: free,
        get_real,
        set_real,
        get_integer,
        set_integer,
        get_boolean,
        set_boolean,
        get_string,
        set_string,
    }
}

fn reset() {
    for counter in [
        &SETUP_COUNT,
        &ENTER_COUNT,
        &EXIT_COUNT,
        &STEP_COUNT,
        &TERMINATE_COUNT,
        &FREE_COUNT,
    ] {
        counter.store(0, Ordering::SeqCst);
    }
}

#[test]
fn checked_constructor_rejects_null_component() {
    let result =
        unsafe { Fmi2CoSimulationInstance::from_raw_parts_checked(ptr::null_mut(), table()) };

    match result {
        Err(error) => assert_eq!(error, FmiError::NullComponent),
        Ok(_) => panic!("null component accepted"),
    }
}

#[test]
fn fmi2_lifecycle_calls_are_ordered_and_cleanup_is_idempotent() {
    reset();
    let mut component_marker = 7usize;
    let component = (&mut component_marker as *mut usize).cast::<c_void>();

    {
        let mut instance = unsafe {
            Fmi2CoSimulationInstance::from_raw_parts_checked(component, table()).unwrap()
        };
        assert_eq!(instance.lifecycle_state(), Fmi2LifecycleState::Instantiated);

        instance.setup_experiment(None, 0.0, Some(1.0)).unwrap();
        instance.enter_initialization_mode().unwrap();
        assert_eq!(
            instance.lifecycle_state(),
            Fmi2LifecycleState::InitializationMode
        );

        instance.exit_initialization_mode().unwrap();
        assert_eq!(instance.lifecycle_state(), Fmi2LifecycleState::StepMode);

        instance.do_step(0.0, 0.001).unwrap();
        instance.terminate().unwrap();
        instance.terminate().unwrap();
        assert_eq!(instance.lifecycle_state(), Fmi2LifecycleState::Terminated);
    }

    assert_eq!(SETUP_COUNT.load(Ordering::SeqCst), 1);
    assert_eq!(ENTER_COUNT.load(Ordering::SeqCst), 1);
    assert_eq!(EXIT_COUNT.load(Ordering::SeqCst), 1);
    assert_eq!(STEP_COUNT.load(Ordering::SeqCst), 1);
    assert_eq!(TERMINATE_COUNT.load(Ordering::SeqCst), 1);
    assert_eq!(FREE_COUNT.load(Ordering::SeqCst), 1);
}
