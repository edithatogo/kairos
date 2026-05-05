pub mod fmu_loader;

#[cfg(feature = "fmi2")]
pub mod fmi2;

#[cfg(feature = "fmi2")]
pub mod instance;

#[cfg(feature = "fmi3")]
pub mod fmi3;
