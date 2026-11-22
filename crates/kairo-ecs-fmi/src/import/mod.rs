pub mod fmu_loader;

#[cfg(any(feature = "fmi2", feature = "fmi3"))]
mod native_state;

#[cfg(feature = "fmi-runtime")]
pub mod dynamic_loader;

#[cfg(feature = "fmi2")]
pub mod fmi2;

#[cfg(feature = "fmi2")]
pub mod instance;

#[cfg(feature = "fmi3")]
pub mod fmi3;
