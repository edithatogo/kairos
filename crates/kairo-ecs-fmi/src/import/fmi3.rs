use crate::{FmiError, FmiResult};
use std::ffi::{c_char, c_void};

pub type Fmi3Instance = *mut c_void;
pub type Fmi3ValueReference = u32;
pub type Fmi3Float64 = f64;
pub type Fmi3Int32 = i32;
pub type Fmi3Boolean = bool;
pub type Fmi3String = *const c_char;

#[repr(i32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Fmi3Status {
    Ok = 0,
    Warning = 1,
    Discard = 2,
    Error = 3,
    Fatal = 4,
}

impl Fmi3Status {
    pub fn is_success(self) -> bool {
        matches!(self, Self::Ok | Self::Warning)
    }
}

pub type Fmi3InstantiateCoSimulation = unsafe extern "C" fn(
    *const c_char,
    *const c_char,
    *const c_char,
    Fmi3Boolean,
    Fmi3Boolean,
    Fmi3Boolean,
    Fmi3Boolean,
    *const Fmi3ValueReference,
    usize,
    *const c_void,
    *const c_void,
    *const c_void,
) -> Fmi3Instance;

pub type Fmi3EnterInitializationMode = unsafe extern "C" fn(
    Fmi3Instance,
    Fmi3Boolean,
    Fmi3Float64,
    Fmi3Float64,
    Fmi3Boolean,
    Fmi3Float64,
) -> i32;
pub type Fmi3ExitInitializationMode = unsafe extern "C" fn(Fmi3Instance) -> i32;
pub type Fmi3DoStep = unsafe extern "C" fn(
    Fmi3Instance,
    Fmi3Float64,
    Fmi3Float64,
    Fmi3Boolean,
    *mut Fmi3Boolean,
    *mut Fmi3Boolean,
    *mut Fmi3Float64,
) -> i32;
pub type Fmi3Terminate = unsafe extern "C" fn(Fmi3Instance) -> i32;
pub type Fmi3FreeInstance = unsafe extern "C" fn(Fmi3Instance);
pub type Fmi3GetFloat64 = unsafe extern "C" fn(
    Fmi3Instance,
    *const Fmi3ValueReference,
    usize,
    *mut Fmi3Float64,
    usize,
) -> i32;
pub type Fmi3SetFloat64 = unsafe extern "C" fn(
    Fmi3Instance,
    *const Fmi3ValueReference,
    usize,
    *const Fmi3Float64,
    usize,
) -> i32;

#[derive(Clone, Copy)]
pub struct Fmi3FunctionTable {
    pub instantiate_co_simulation: Option<Fmi3InstantiateCoSimulation>,
    pub enter_initialization_mode: Fmi3EnterInitializationMode,
    pub exit_initialization_mode: Fmi3ExitInitializationMode,
    pub do_step: Fmi3DoStep,
    pub terminate: Fmi3Terminate,
    pub free_instance: Fmi3FreeInstance,
    pub get_float64: Fmi3GetFloat64,
    pub set_float64: Fmi3SetFloat64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Fmi3LifecycleState {
    Instantiated,
    InitializationMode,
    StepMode,
    Terminated,
}

pub struct Fmi3CoSimulationInstance {
    instance: Fmi3Instance,
    table: Fmi3FunctionTable,
    lifecycle_state: Fmi3LifecycleState,
}

impl Fmi3CoSimulationInstance {
    /// Builds an FMI 3 co-simulation instance after verifying the pointer is non-null.
    ///
    /// # Safety
    ///
    /// `instance` must be a valid FMI 3 instance pointer created by the matching
    /// dynamic library and function table, and must remain valid for the duration of
    /// this wrapper.
    pub unsafe fn from_raw_parts_checked(
        instance: Fmi3Instance,
        table: Fmi3FunctionTable,
    ) -> FmiResult<Self> {
        if instance.is_null() {
            Err(FmiError::NullComponent)
        } else {
            Ok(Self {
                instance,
                table,
                lifecycle_state: Fmi3LifecycleState::Instantiated,
            })
        }
    }

    pub fn lifecycle_state(&self) -> Fmi3LifecycleState {
        self.lifecycle_state
    }

    pub fn enter_initialization_mode(
        &mut self,
        tolerance: Option<f64>,
        start_time: f64,
        stop_time: Option<f64>,
    ) -> FmiResult<()> {
        let status = unsafe {
            (self.table.enter_initialization_mode)(
                self.instance,
                tolerance.is_some(),
                tolerance.unwrap_or(0.0),
                start_time,
                stop_time.is_some(),
                stop_time.unwrap_or(0.0),
            )
        };
        if status == 0 {
            self.lifecycle_state = Fmi3LifecycleState::InitializationMode;
            Ok(())
        } else {
            Err(FmiError::FmiStatus {
                operation: "fmi3EnterInitializationMode",
                status,
            })
        }
    }

    pub fn exit_initialization_mode(&mut self) -> FmiResult<()> {
        let status = unsafe { (self.table.exit_initialization_mode)(self.instance) };
        if status == 0 {
            self.lifecycle_state = Fmi3LifecycleState::StepMode;
            Ok(())
        } else {
            Err(FmiError::FmiStatus {
                operation: "fmi3ExitInitializationMode",
                status,
            })
        }
    }

    pub fn do_step(&mut self, current_time: f64, step_size: f64) -> FmiResult<()> {
        let mut event_handling_needed = false;
        let mut terminate_simulation = false;
        let mut early_return_time = 0.0;
        let status = unsafe {
            (self.table.do_step)(
                self.instance,
                current_time,
                step_size,
                true,
                &mut event_handling_needed,
                &mut terminate_simulation,
                &mut early_return_time,
            )
        };
        if status == 0 {
            Ok(())
        } else {
            Err(FmiError::FmiStatus {
                operation: "fmi3DoStep",
                status,
            })
        }
    }

    pub fn get_float64(&self, references: &[Fmi3ValueReference]) -> FmiResult<Vec<f64>> {
        let mut values = vec![0.0; references.len()];
        let status = unsafe {
            (self.table.get_float64)(
                self.instance,
                references.as_ptr(),
                references.len(),
                values.as_mut_ptr(),
                values.len(),
            )
        };
        if status == 0 {
            Ok(values)
        } else {
            Err(FmiError::FmiStatus {
                operation: "fmi3GetFloat64",
                status,
            })
        }
    }

    pub fn terminate(&mut self) -> FmiResult<()> {
        let status = unsafe { (self.table.terminate)(self.instance) };
        self.lifecycle_state = Fmi3LifecycleState::Terminated;
        if status == 0 {
            Ok(())
        } else {
            Err(FmiError::FmiStatus {
                operation: "fmi3Terminate",
                status,
            })
        }
    }
}

impl Drop for Fmi3CoSimulationInstance {
    fn drop(&mut self) {
        if !self.instance.is_null() {
            unsafe {
                (self.table.free_instance)(self.instance);
            }
        }
    }
}
