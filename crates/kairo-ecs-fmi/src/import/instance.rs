use crate::import::fmi2::{
    Fmi2Boolean, Fmi2Callbacks, Fmi2Component, Fmi2FunctionTable, Fmi2Integer, Fmi2Real,
    Fmi2String, Fmi2ValueReference,
};
use crate::import::native_state::{NativeCallState, NativeFmuState};
use crate::{FmiError, FmiResult};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub trait FmuInstance {
    fn enter_initialization_mode(&mut self) -> FmiResult<()>;
    fn exit_initialization_mode(&mut self) -> FmiResult<()>;
    fn do_step(&mut self, current_time: f64, step_size: f64) -> FmiResult<()>;
    fn terminate(&mut self) -> FmiResult<()>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Fmi2LifecycleState {
    Instantiated,
    InitializationMode,
    StepMode,
    Terminated,
}

pub struct Fmi2CoSimulationInstance {
    component: Fmi2Component,
    table: Fmi2FunctionTable,
    lifecycle_state: Fmi2LifecycleState,
    _library: Option<Arc<libloading::Library>>,
    _callbacks: Option<Box<Fmi2Callbacks>>,
    saved_state: Option<NativeFmuState>,
    calls: NativeCallState,
}

impl Fmi2CoSimulationInstance {
    /// Builds an FMI 2 co-simulation wrapper from an already-instantiated FMI component.
    ///
    /// # Safety
    ///
    /// `component` must be a valid FMI component created by the same FMU function table,
    /// and every function pointer in `table` must remain valid for the lifetime of the
    /// returned wrapper. The wrapper will call `terminate` and `free_instance` during drop
    /// when appropriate, so callers must not free the component elsewhere.
    pub unsafe fn from_raw_parts(component: Fmi2Component, table: Fmi2FunctionTable) -> Self {
        Self {
            component,
            table,
            lifecycle_state: Fmi2LifecycleState::Instantiated,
            _library: None,
            _callbacks: None,
            saved_state: None,
            calls: NativeCallState::default(),
        }
    }

    /// Builds an FMI 2 co-simulation wrapper after rejecting a null component.
    ///
    /// # Safety
    ///
    /// `component` must be a valid FMI component created by the same FMU function table,
    /// and every function pointer in `table` must remain valid for the lifetime of the
    /// returned wrapper.
    pub unsafe fn from_raw_parts_checked(
        component: Fmi2Component,
        table: Fmi2FunctionTable,
    ) -> FmiResult<Self> {
        if component.is_null() {
            Err(FmiError::NullComponent)
        } else {
            Ok(Self::from_raw_parts(component, table))
        }
    }

    pub fn lifecycle_state(&self) -> Fmi2LifecycleState {
        if self.calls.failed() || !self.calls.can_call() {
            Fmi2LifecycleState::Terminated
        } else {
            self.lifecycle_state
        }
    }

    pub(crate) fn retain_library(
        &mut self,
        library: Arc<libloading::Library>,
        fatal: Arc<AtomicBool>,
        callbacks: Box<Fmi2Callbacks>,
        saved_state: Option<NativeFmuState>,
    ) {
        self._library = Some(library);
        self._callbacks = Some(callbacks);
        self.saved_state = saved_state;
        self.calls.fatal = fatal;
    }

    /// Capture one in-process state while in Step Mode.
    pub fn save_state(&mut self) -> FmiResult<()> {
        self.calls.check_active()?;
        if self.lifecycle_state != Fmi2LifecycleState::StepMode {
            return Err(FmiError::InvalidInput(
                "FMU state requires Step Mode".into(),
            ));
        }
        let result = self
            .saved_state
            .as_mut()
            .ok_or(FmiError::UnsupportedCapability(
                "FMI 2 state capture and restore",
            ))?
            .save(self.component);
        self.calls.observe(result)
    }

    /// Restore the captured state. The caller resumes at its saved communication point.
    pub fn restore_state(&mut self) -> FmiResult<()> {
        self.calls.check_liveness()?;
        if self.lifecycle_state != Fmi2LifecycleState::StepMode {
            return Err(FmiError::InvalidInput(
                "FMU state requires Step Mode".into(),
            ));
        }
        let result = self
            .saved_state
            .as_ref()
            .ok_or(FmiError::UnsupportedCapability(
                "FMI 2 state capture and restore",
            ))?
            .restore(self.component);
        self.calls.observe(result)?;
        self.calls.recovered();
        Ok(())
    }

    /// Release the saved state. A later call to `save_state` can capture a new one.
    pub fn discard_state(&mut self) -> FmiResult<()> {
        self.calls.check_liveness()?;
        if let Some(state) = &mut self.saved_state {
            self.calls.observe(state.clear(self.component))?;
        }
        Ok(())
    }

    pub fn setup_experiment(
        &mut self,
        tolerance: Option<f64>,
        start_time: f64,
        stop_time: Option<f64>,
    ) -> FmiResult<()> {
        self.calls.check_active()?;
        let tolerance_defined = i32::from(tolerance.is_some());
        let stop_time_defined = i32::from(stop_time.is_some());
        let status = unsafe {
            (self.table.setup_experiment)(
                self.component,
                tolerance_defined,
                tolerance.unwrap_or_default(),
                start_time,
                stop_time_defined,
                stop_time.unwrap_or_default(),
            )
        };
        self.calls.accept("fmi2SetupExperiment", status)
    }

    pub fn get_real(&self, references: &[Fmi2ValueReference]) -> FmiResult<Vec<Fmi2Real>> {
        self.calls.check_active()?;
        let mut values = vec![0.0; references.len()];
        let status = unsafe {
            (self.table.get_real)(
                self.component,
                references.as_ptr(),
                references.len(),
                values.as_mut_ptr(),
            )
        };
        self.calls.accept("fmi2GetReal", status)?;
        Ok(values)
    }

    pub fn set_real(
        &mut self,
        references: &[Fmi2ValueReference],
        values: &[Fmi2Real],
    ) -> FmiResult<()> {
        self.calls.check_active()?;
        ensure_matching_lengths(references.len(), values.len())?;
        let status = unsafe {
            (self.table.set_real)(
                self.component,
                references.as_ptr(),
                references.len(),
                values.as_ptr(),
            )
        };
        self.calls.accept("fmi2SetReal", status)
    }

    pub fn get_integer(&self, references: &[Fmi2ValueReference]) -> FmiResult<Vec<Fmi2Integer>> {
        self.calls.check_active()?;
        let mut values = vec![0; references.len()];
        let status = unsafe {
            (self.table.get_integer)(
                self.component,
                references.as_ptr(),
                references.len(),
                values.as_mut_ptr(),
            )
        };
        self.calls.accept("fmi2GetInteger", status)?;
        Ok(values)
    }

    pub fn set_integer(
        &mut self,
        references: &[Fmi2ValueReference],
        values: &[Fmi2Integer],
    ) -> FmiResult<()> {
        self.calls.check_active()?;
        ensure_matching_lengths(references.len(), values.len())?;
        let status = unsafe {
            (self.table.set_integer)(
                self.component,
                references.as_ptr(),
                references.len(),
                values.as_ptr(),
            )
        };
        self.calls.accept("fmi2SetInteger", status)
    }

    pub fn get_boolean(&self, references: &[Fmi2ValueReference]) -> FmiResult<Vec<bool>> {
        self.calls.check_active()?;
        let mut values = vec![0; references.len()];
        let status = unsafe {
            (self.table.get_boolean)(
                self.component,
                references.as_ptr(),
                references.len(),
                values.as_mut_ptr(),
            )
        };
        self.calls.accept("fmi2GetBoolean", status)?;
        Ok(values.into_iter().map(|value| value != 0).collect())
    }

    pub fn set_boolean(
        &mut self,
        references: &[Fmi2ValueReference],
        values: &[bool],
    ) -> FmiResult<()> {
        self.calls.check_active()?;
        ensure_matching_lengths(references.len(), values.len())?;
        let raw: Vec<Fmi2Boolean> = values.iter().map(|value| i32::from(*value)).collect();
        let status = unsafe {
            (self.table.set_boolean)(
                self.component,
                references.as_ptr(),
                references.len(),
                raw.as_ptr(),
            )
        };
        self.calls.accept("fmi2SetBoolean", status)
    }

    pub fn get_string(&self, references: &[Fmi2ValueReference]) -> FmiResult<Vec<Fmi2String>> {
        self.calls.check_active()?;
        let mut values = vec![std::ptr::null(); references.len()];
        let status = unsafe {
            (self.table.get_string)(
                self.component,
                references.as_ptr(),
                references.len(),
                values.as_mut_ptr(),
            )
        };
        self.calls.accept("fmi2GetString", status)?;
        Ok(values)
    }

    pub fn set_string(
        &mut self,
        references: &[Fmi2ValueReference],
        values: &[Fmi2String],
    ) -> FmiResult<()> {
        self.calls.check_active()?;
        ensure_matching_lengths(references.len(), values.len())?;
        let status = unsafe {
            (self.table.set_string)(
                self.component,
                references.as_ptr(),
                references.len(),
                values.as_ptr(),
            )
        };
        self.calls.accept("fmi2SetString", status)
    }
}

impl FmuInstance for Fmi2CoSimulationInstance {
    fn enter_initialization_mode(&mut self) -> FmiResult<()> {
        self.calls.check_active()?;
        let status = unsafe { (self.table.enter_initialization_mode)(self.component) };
        self.calls.accept("fmi2EnterInitializationMode", status)?;
        self.lifecycle_state = Fmi2LifecycleState::InitializationMode;
        Ok(())
    }

    fn exit_initialization_mode(&mut self) -> FmiResult<()> {
        self.calls.check_active()?;
        let status = unsafe { (self.table.exit_initialization_mode)(self.component) };
        self.calls.accept("fmi2ExitInitializationMode", status)?;
        self.lifecycle_state = Fmi2LifecycleState::StepMode;
        Ok(())
    }

    fn do_step(&mut self, current_time: f64, step_size: f64) -> FmiResult<()> {
        self.calls.check_active()?;
        if self.lifecycle_state != Fmi2LifecycleState::StepMode {
            return Err(FmiError::InvalidInput("FMU step requires Step Mode".into()));
        }
        let no_set_fmu_state_prior = i32::from(self.saved_state.is_none());
        let status = unsafe {
            (self.table.do_step)(
                self.component,
                current_time,
                step_size,
                no_set_fmu_state_prior,
            )
        };
        self.calls.accept("fmi2DoStep", status)
    }

    fn terminate(&mut self) -> FmiResult<()> {
        self.calls.check_liveness()?;
        if self.lifecycle_state == Fmi2LifecycleState::Terminated {
            return Ok(());
        }
        self.discard_state()?;
        if !self.calls.failed() {
            let status = unsafe { (self.table.terminate)(self.component) };
            self.calls.accept("fmi2Terminate", status)?;
        }
        self.lifecycle_state = Fmi2LifecycleState::Terminated;
        Ok(())
    }
}

impl Drop for Fmi2CoSimulationInstance {
    fn drop(&mut self) {
        if self.calls.is_pending() {
            // An unexpected asynchronous step may still use both. Keep them
            // alive until process exit because this wrapper has no cancel API.
            std::mem::forget(self._library.take());
            std::mem::forget(self._callbacks.take());
            return;
        }
        if self.lifecycle_state == Fmi2LifecycleState::StepMode || self.calls.failed() {
            let _ = self.terminate();
        }
        if self.calls.can_call() {
            unsafe { (self.table.free_instance)(self.component) };
        }
    }
}

fn ensure_matching_lengths(expected: usize, actual: usize) -> FmiResult<()> {
    if expected == actual {
        Ok(())
    } else {
        Err(FmiError::InvalidVariableCount { expected, actual })
    }
}
