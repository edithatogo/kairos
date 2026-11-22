use crate::error::validation_error;
use crate::import::native_state::{NativeCallState, NativeFmuState};
use crate::{FmiError, FmiResult};
use std::ffi::{c_char, c_void};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

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
    _library: Option<Arc<libloading::Library>>,
    saved_state: Option<NativeFmuState>,
    calls: NativeCallState,
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
                _library: None,
                saved_state: None,
                calls: NativeCallState::default(),
            })
        }
    }

    pub fn lifecycle_state(&self) -> Fmi3LifecycleState {
        if self.calls.failed() || !self.calls.can_call() {
            Fmi3LifecycleState::Terminated
        } else {
            self.lifecycle_state
        }
    }

    pub(crate) fn retain_library(
        &mut self,
        library: Arc<libloading::Library>,
        fatal: Arc<AtomicBool>,
        saved_state: Option<NativeFmuState>,
    ) {
        self._library = Some(library);
        self.saved_state = saved_state;
        self.calls.fatal = fatal;
    }

    /// Capture one in-process state while in Step Mode.
    pub fn save_state(&mut self) -> FmiResult<()> {
        self.calls.check_active()?;
        if self.lifecycle_state != Fmi3LifecycleState::StepMode {
            return Err(FmiError::InvalidInput(
                "FMU state requires Step Mode".into(),
            ));
        }
        let result = self
            .saved_state
            .as_mut()
            .ok_or(FmiError::UnsupportedCapability(
                "FMI 3 state capture and restore",
            ))?
            .save(self.instance);
        self.calls.observe(result)
    }

    /// Restore the captured state. The caller resumes at its saved communication point.
    pub fn restore_state(&mut self) -> FmiResult<()> {
        self.calls.check_liveness()?;
        if self.lifecycle_state != Fmi3LifecycleState::StepMode {
            return Err(FmiError::InvalidInput(
                "FMU state requires Step Mode".into(),
            ));
        }
        let result = self
            .saved_state
            .as_ref()
            .ok_or(FmiError::UnsupportedCapability(
                "FMI 3 state capture and restore",
            ))?
            .restore(self.instance);
        self.calls.observe(result)?;
        self.calls.recovered();
        Ok(())
    }

    /// Release the saved state. A later call to `save_state` can capture a new one.
    pub fn discard_state(&mut self) -> FmiResult<()> {
        self.calls.check_liveness()?;
        if let Some(state) = &mut self.saved_state {
            self.calls.observe(state.clear(self.instance))?;
        }
        Ok(())
    }

    pub fn enter_initialization_mode(
        &mut self,
        tolerance: Option<f64>,
        start_time: f64,
        stop_time: Option<f64>,
    ) -> FmiResult<()> {
        self.calls.check_active()?;
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
        self.calls.accept("fmi3EnterInitializationMode", status)?;
        self.lifecycle_state = Fmi3LifecycleState::InitializationMode;
        Ok(())
    }

    pub fn exit_initialization_mode(&mut self) -> FmiResult<()> {
        self.calls.check_active()?;
        let status = unsafe { (self.table.exit_initialization_mode)(self.instance) };
        self.calls.accept("fmi3ExitInitializationMode", status)?;
        self.lifecycle_state = Fmi3LifecycleState::StepMode;
        Ok(())
    }

    pub fn do_step(&mut self, current_time: f64, step_size: f64) -> FmiResult<()> {
        self.calls.check_active()?;
        if self.lifecycle_state != Fmi3LifecycleState::StepMode {
            return Err(FmiError::InvalidInput("FMU step requires Step Mode".into()));
        }
        let requested_end = current_time + step_size;
        if !current_time.is_finite() || !requested_end.is_finite() || requested_end <= current_time
        {
            return Err(FmiError::InvalidInput(
                "FMI 3 step must advance to a finite time".into(),
            ));
        }
        let mut event_handling_needed = false;
        let mut terminate_simulation = false;
        let mut early_return = false;
        let mut last_successful_time = current_time;
        let status = unsafe {
            (self.table.do_step)(
                self.instance,
                current_time,
                step_size,
                self.saved_state.is_none(),
                &mut event_handling_needed,
                &mut terminate_simulation,
                &mut early_return,
                &mut last_successful_time,
            )
        };
        self.calls.accept("fmi3DoStep", status)?;
        if event_handling_needed || terminate_simulation || early_return {
            return Err(validation_error(
                "FMI 3 co-simulation step",
                format!(
                    "FMU stopped at {last_successful_time} with eventHandlingNeeded={event_handling_needed}, terminateSimulation={terminate_simulation}, earlyReturn={early_return}"
                ),
            ));
        }
        // The unit-returning API promises a full communication step. FMI 3
        // can also report a different reached time with all flags false.
        let rounding = 8.0
            * f64::EPSILON
            * current_time
                .abs()
                .max(step_size.abs())
                .max(requested_end.abs());
        if !last_successful_time.is_finite()
            || last_successful_time <= current_time
            || (last_successful_time - requested_end).abs() > rounding
        {
            return Err(validation_error(
                "FMI 3 co-simulation step",
                format!("FMU reached {last_successful_time} instead of {requested_end}"),
            ));
        }
        Ok(())
    }

    pub fn get_float64(&self, references: &[Fmi3ValueReference]) -> FmiResult<Vec<f64>> {
        self.calls.check_active()?;
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
        self.calls.accept("fmi3GetFloat64", status)?;
        Ok(values)
    }

    pub fn terminate(&mut self) -> FmiResult<()> {
        self.calls.check_liveness()?;
        if self.lifecycle_state == Fmi3LifecycleState::Terminated {
            return Ok(());
        }
        self.discard_state()?;
        if !self.calls.failed() {
            let status = unsafe { (self.table.terminate)(self.instance) };
            self.calls.accept("fmi3Terminate", status)?;
        }
        self.lifecycle_state = Fmi3LifecycleState::Terminated;
        Ok(())
    }
}

impl Drop for Fmi3CoSimulationInstance {
    fn drop(&mut self) {
        if self.lifecycle_state == Fmi3LifecycleState::StepMode || self.calls.failed() {
            let _ = self.terminate();
        }
        if self.calls.can_call() {
            unsafe { (self.table.free_instance)(self.instance) };
        }
    }
}
