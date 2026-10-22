use std::path::Path;
use std::sync::Arc;

use crate::import::fmi2::{
    Fmi2Boolean, Fmi2Component, Fmi2DoStep, Fmi2EnterInitializationMode,
    Fmi2ExitInitializationMode, Fmi2FreeInstance, Fmi2FunctionTable, Fmi2GetBoolean,
    Fmi2GetInteger, Fmi2GetReal, Fmi2GetString, Fmi2SetBoolean, Fmi2SetInteger, Fmi2SetReal,
    Fmi2SetString, Fmi2SetupExperiment, Fmi2Terminate,
};
use crate::import::fmi3::{
    Fmi3CoSimulationInstance, Fmi3DoStep, Fmi3EnterInitializationMode, Fmi3ExitInitializationMode,
    Fmi3FreeInstance, Fmi3FunctionTable, Fmi3GetFloat64, Fmi3InstantiateCoSimulation,
    Fmi3SetFloat64, Fmi3Terminate,
};
use crate::import::instance::Fmi2CoSimulationInstance;
use crate::{FmiError, FmiResult};

pub type Fmi2Instantiate = unsafe extern "C" fn(
    *const std::ffi::c_char,
    i32,
    *const std::ffi::c_char,
    *const std::ffi::c_char,
    *const std::ffi::c_void,
    Fmi2Boolean,
    Fmi2Boolean,
) -> Fmi2Component;

pub struct LoadedFmuLibrary {
    #[allow(dead_code)]
    lib: Arc<libloading::Library>,
    pub table: Fmi2FunctionTable,
    pub instantiate_fn: Option<Fmi2Instantiate>,
}

impl LoadedFmuLibrary {
    pub fn load_from_path(dylib_path: &Path) -> FmiResult<Self> {
        let lib = unsafe {
            libloading::Library::new(dylib_path).map_err(|e| {
                FmiError::LibraryLoadFailed(format!("failed to load {}: {e}", dylib_path.display()))
            })?
        };

        unsafe {
            let setup_experiment: libloading::Symbol<Fmi2SetupExperiment> = lib
                .get(b"fmi2SetupExperiment\0")
                .map_err(|e| FmiError::SymbolNotFound(e.to_string()))?;
            let enter_init: libloading::Symbol<Fmi2EnterInitializationMode> = lib
                .get(b"fmi2EnterInitializationMode\0")
                .map_err(|e| FmiError::SymbolNotFound(e.to_string()))?;
            let exit_init: libloading::Symbol<Fmi2ExitInitializationMode> = lib
                .get(b"fmi2ExitInitializationMode\0")
                .map_err(|e| FmiError::SymbolNotFound(e.to_string()))?;
            let do_step: libloading::Symbol<Fmi2DoStep> = lib
                .get(b"fmi2DoStep\0")
                .map_err(|e| FmiError::SymbolNotFound(e.to_string()))?;
            let terminate: libloading::Symbol<Fmi2Terminate> = lib
                .get(b"fmi2Terminate\0")
                .map_err(|e| FmiError::SymbolNotFound(e.to_string()))?;
            let free_instance: libloading::Symbol<Fmi2FreeInstance> = lib
                .get(b"fmi2FreeInstance\0")
                .map_err(|e| FmiError::SymbolNotFound(e.to_string()))?;
            let get_real: libloading::Symbol<Fmi2GetReal> = lib
                .get(b"fmi2GetReal\0")
                .map_err(|e| FmiError::SymbolNotFound(e.to_string()))?;
            let set_real: libloading::Symbol<Fmi2SetReal> = lib
                .get(b"fmi2SetReal\0")
                .map_err(|e| FmiError::SymbolNotFound(e.to_string()))?;
            let get_integer: libloading::Symbol<Fmi2GetInteger> = lib
                .get(b"fmi2GetInteger\0")
                .map_err(|e| FmiError::SymbolNotFound(e.to_string()))?;
            let set_integer: libloading::Symbol<Fmi2SetInteger> = lib
                .get(b"fmi2SetInteger\0")
                .map_err(|e| FmiError::SymbolNotFound(e.to_string()))?;
            let get_boolean: libloading::Symbol<Fmi2GetBoolean> = lib
                .get(b"fmi2GetBoolean\0")
                .map_err(|e| FmiError::SymbolNotFound(e.to_string()))?;
            let set_boolean: libloading::Symbol<Fmi2SetBoolean> = lib
                .get(b"fmi2SetBoolean\0")
                .map_err(|e| FmiError::SymbolNotFound(e.to_string()))?;
            let get_string: libloading::Symbol<Fmi2GetString> = lib
                .get(b"fmi2GetString\0")
                .map_err(|e| FmiError::SymbolNotFound(e.to_string()))?;
            let set_string: libloading::Symbol<Fmi2SetString> = lib
                .get(b"fmi2SetString\0")
                .map_err(|e| FmiError::SymbolNotFound(e.to_string()))?;

            let instantiate_fn = lib.get(b"fmi2Instantiate\0").ok().map(|s| *s);

            let table = Fmi2FunctionTable {
                setup_experiment: *setup_experiment,
                enter_initialization_mode: *enter_init,
                exit_initialization_mode: *exit_init,
                do_step: *do_step,
                terminate: *terminate,
                free_instance: *free_instance,
                get_real: *get_real,
                set_real: *set_real,
                get_integer: *get_integer,
                set_integer: *set_integer,
                get_boolean: *get_boolean,
                set_boolean: *set_boolean,
                get_string: *get_string,
                set_string: *set_string,
            };

            Ok(Self {
                lib: Arc::new(lib),
                table,
                instantiate_fn,
            })
        }
    }

    pub fn instantiate_checked(
        &self,
        instance_name: &str,
        resource_location: &str,
    ) -> FmiResult<Fmi2CoSimulationInstance> {
        let instantiate = self
            .instantiate_fn
            .ok_or_else(|| FmiError::SymbolNotFound("fmi2Instantiate not found".to_string()))?;

        let c_instance_name = std::ffi::CString::new(instance_name)
            .map_err(|_| FmiError::InvalidInput("invalid instance_name".to_string()))?;
        let c_guid = std::ffi::CString::new("{12345678-1234-1234-1234-123456789abc}")
            .map_err(|_| FmiError::InvalidInput("invalid guid".to_string()))?;
        let c_res_loc = std::ffi::CString::new(resource_location)
            .map_err(|_| FmiError::InvalidInput("invalid resource_location".to_string()))?;

        let component = unsafe {
            instantiate(
                c_instance_name.as_ptr(),
                1, // fmi2CoSimulation
                c_guid.as_ptr(),
                c_res_loc.as_ptr(),
                std::ptr::null(),
                0, // visible
                0, // loggingOn
            )
        };

        if component.is_null() {
            return Err(FmiError::NullComponent);
        }

        unsafe { Fmi2CoSimulationInstance::from_raw_parts_checked(component, self.table) }
    }

    pub fn load_fmi3_from_path(dylib_path: &Path) -> FmiResult<LoadedFmi3Library> {
        LoadedFmi3Library::load_from_path(dylib_path)
    }
}

pub struct LoadedFmi3Library {
    #[allow(dead_code)]
    lib: Arc<libloading::Library>,
    pub table: Fmi3FunctionTable,
}

impl LoadedFmi3Library {
    pub fn load_from_path(dylib_path: &Path) -> FmiResult<Self> {
        let lib = unsafe {
            libloading::Library::new(dylib_path).map_err(|e| {
                FmiError::LibraryLoadFailed(format!("failed to load {}: {e}", dylib_path.display()))
            })?
        };

        unsafe {
            let enter_init: libloading::Symbol<Fmi3EnterInitializationMode> = lib
                .get(b"fmi3EnterInitializationMode\0")
                .map_err(|e| FmiError::SymbolNotFound(e.to_string()))?;
            let exit_init: libloading::Symbol<Fmi3ExitInitializationMode> = lib
                .get(b"fmi3ExitInitializationMode\0")
                .map_err(|e| FmiError::SymbolNotFound(e.to_string()))?;
            let do_step: libloading::Symbol<Fmi3DoStep> = lib
                .get(b"fmi3DoStep\0")
                .map_err(|e| FmiError::SymbolNotFound(e.to_string()))?;
            let terminate: libloading::Symbol<Fmi3Terminate> = lib
                .get(b"fmi3Terminate\0")
                .map_err(|e| FmiError::SymbolNotFound(e.to_string()))?;
            let free_instance: libloading::Symbol<Fmi3FreeInstance> = lib
                .get(b"fmi3FreeInstance\0")
                .map_err(|e| FmiError::SymbolNotFound(e.to_string()))?;
            let get_float64: libloading::Symbol<Fmi3GetFloat64> = lib
                .get(b"fmi3GetFloat64\0")
                .map_err(|e| FmiError::SymbolNotFound(e.to_string()))?;
            let set_float64: libloading::Symbol<Fmi3SetFloat64> = lib
                .get(b"fmi3SetFloat64\0")
                .map_err(|e| FmiError::SymbolNotFound(e.to_string()))?;

            let instantiate_co_simulation: Option<Fmi3InstantiateCoSimulation> =
                lib.get(b"fmi3InstantiateCoSimulation\0").ok().map(|s| *s);

            let table = Fmi3FunctionTable {
                instantiate_co_simulation,
                enter_initialization_mode: *enter_init,
                exit_initialization_mode: *exit_init,
                do_step: *do_step,
                terminate: *terminate,
                free_instance: *free_instance,
                get_float64: *get_float64,
                set_float64: *set_float64,
            };

            Ok(Self {
                lib: Arc::new(lib),
                table,
            })
        }
    }

    pub fn instantiate_checked(
        &self,
        instance_name: &str,
        resource_location: &str,
    ) -> FmiResult<Fmi3CoSimulationInstance> {
        let instantiate = self.table.instantiate_co_simulation.ok_or_else(|| {
            FmiError::SymbolNotFound("fmi3InstantiateCoSimulation not found".to_string())
        })?;

        let c_instance_name = std::ffi::CString::new(instance_name)
            .map_err(|_| FmiError::InvalidInput("invalid instance_name".to_string()))?;
        let c_guid = std::ffi::CString::new("{12345678-1234-1234-1234-123456789abc}")
            .map_err(|_| FmiError::InvalidInput("invalid token".to_string()))?;
        let c_res_loc = std::ffi::CString::new(resource_location)
            .map_err(|_| FmiError::InvalidInput("invalid resource_location".to_string()))?;

        let component = unsafe {
            instantiate(
                c_instance_name.as_ptr(),
                c_guid.as_ptr(),
                c_res_loc.as_ptr(),
                false,
                false,
                false,
                false,
                std::ptr::null(),
                0,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
            )
        };

        if component.is_null() {
            return Err(FmiError::NullComponent);
        }

        unsafe { Fmi3CoSimulationInstance::from_raw_parts_checked(component, self.table) }
    }
}
