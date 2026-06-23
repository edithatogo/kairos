use crate::import::fmi2::{
    Fmi2Boolean, Fmi2Component, Fmi2FunctionTable, Fmi2Integer, Fmi2Real, Fmi2Status, Fmi2String,
    Fmi2ValueReference,
};
use crate::{FmiError, FmiResult};

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
        self.lifecycle_state
    }

    pub fn setup_experiment(
        &mut self,
        tolerance: Option<f64>,
        start_time: f64,
        stop_time: Option<f64>,
    ) -> FmiResult<()> {
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
        require_success("fmi2SetupExperiment", status)
    }

    pub fn get_real(&self, references: &[Fmi2ValueReference]) -> FmiResult<Vec<Fmi2Real>> {
        let mut values = vec![0.0; references.len()];
        let status = unsafe {
            (self.table.get_real)(
                self.component,
                references.as_ptr(),
                references.len(),
                values.as_mut_ptr(),
            )
        };
        require_success("fmi2GetReal", status)?;
        Ok(values)
    }

    pub fn set_real(
        &mut self,
        references: &[Fmi2ValueReference],
        values: &[Fmi2Real],
    ) -> FmiResult<()> {
        ensure_matching_lengths(references.len(), values.len())?;
        let status = unsafe {
            (self.table.set_real)(
                self.component,
                references.as_ptr(),
                references.len(),
                values.as_ptr(),
            )
        };
        require_success("fmi2SetReal", status)
    }

    pub fn get_integer(&self, references: &[Fmi2ValueReference]) -> FmiResult<Vec<Fmi2Integer>> {
        let mut values = vec![0; references.len()];
        let status = unsafe {
            (self.table.get_integer)(
                self.component,
                references.as_ptr(),
                references.len(),
                values.as_mut_ptr(),
            )
        };
        require_success("fmi2GetInteger", status)?;
        Ok(values)
    }

    pub fn set_integer(
        &mut self,
        references: &[Fmi2ValueReference],
        values: &[Fmi2Integer],
    ) -> FmiResult<()> {
        ensure_matching_lengths(references.len(), values.len())?;
        let status = unsafe {
            (self.table.set_integer)(
                self.component,
                references.as_ptr(),
                references.len(),
                values.as_ptr(),
            )
        };
        require_success("fmi2SetInteger", status)
    }

    pub fn get_boolean(&self, references: &[Fmi2ValueReference]) -> FmiResult<Vec<bool>> {
        let mut values = vec![0; references.len()];
        let status = unsafe {
            (self.table.get_boolean)(
                self.component,
                references.as_ptr(),
                references.len(),
                values.as_mut_ptr(),
            )
        };
        require_success("fmi2GetBoolean", status)?;
        Ok(values.into_iter().map(|value| value != 0).collect())
    }

    pub fn set_boolean(
        &mut self,
        references: &[Fmi2ValueReference],
        values: &[bool],
    ) -> FmiResult<()> {
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
        require_success("fmi2SetBoolean", status)
    }

    pub fn get_string(&self, references: &[Fmi2ValueReference]) -> FmiResult<Vec<Fmi2String>> {
        let mut values = vec![std::ptr::null(); references.len()];
        let status = unsafe {
            (self.table.get_string)(
                self.component,
                references.as_ptr(),
                references.len(),
                values.as_mut_ptr(),
            )
        };
        require_success("fmi2GetString", status)?;
        Ok(values)
    }

    pub fn set_string(
        &mut self,
        references: &[Fmi2ValueReference],
        values: &[Fmi2String],
    ) -> FmiResult<()> {
        ensure_matching_lengths(references.len(), values.len())?;
        let status = unsafe {
            (self.table.set_string)(
                self.component,
                references.as_ptr(),
                references.len(),
                values.as_ptr(),
            )
        };
        require_success("fmi2SetString", status)
    }
}

impl FmuInstance for Fmi2CoSimulationInstance {
    fn enter_initialization_mode(&mut self) -> FmiResult<()> {
        let status = unsafe { (self.table.enter_initialization_mode)(self.component) };
        require_success("fmi2EnterInitializationMode", status)?;
        self.lifecycle_state = Fmi2LifecycleState::InitializationMode;
        Ok(())
    }

    fn exit_initialization_mode(&mut self) -> FmiResult<()> {
        let status = unsafe { (self.table.exit_initialization_mode)(self.component) };
        require_success("fmi2ExitInitializationMode", status)?;
        self.lifecycle_state = Fmi2LifecycleState::StepMode;
        Ok(())
    }

    fn do_step(&mut self, current_time: f64, step_size: f64) -> FmiResult<()> {
        let no_set_fmu_state_prior = 1;
        let status = unsafe {
            (self.table.do_step)(
                self.component,
                current_time,
                step_size,
                no_set_fmu_state_prior,
            )
        };
        require_success("fmi2DoStep", status)
    }

    fn terminate(&mut self) -> FmiResult<()> {
        if self.lifecycle_state == Fmi2LifecycleState::Terminated {
            return Ok(());
        }
        let status = unsafe { (self.table.terminate)(self.component) };
        require_success("fmi2Terminate", status)?;
        self.lifecycle_state = Fmi2LifecycleState::Terminated;
        Ok(())
    }
}

impl Drop for Fmi2CoSimulationInstance {
    fn drop(&mut self) {
        let _ = self.terminate();
        unsafe {
            (self.table.free_instance)(self.component);
        }
    }
}

fn require_success(operation: &'static str, raw_status: i32) -> FmiResult<()> {
    let status = Fmi2Status::from_raw(raw_status);
    if status.is_success() {
        Ok(())
    } else {
        Err(FmiError::FmiStatus {
            operation,
            status: raw_status,
        })
    }
}

fn ensure_matching_lengths(expected: usize, actual: usize) -> FmiResult<()> {
    if expected == actual {
        Ok(())
    } else {
        Err(FmiError::InvalidVariableCount { expected, actual })
    }
}
