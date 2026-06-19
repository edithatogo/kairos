#![allow(unsafe_code)]

pub mod error;

#[cfg(any(feature = "fmi-runtime", feature = "fmi2", feature = "fmi3"))]
pub mod import;

#[cfg(feature = "fmi2")]
pub mod export;

#[cfg(feature = "aas")]
pub mod aas;

#[cfg(feature = "digital-twin")]
pub mod digital_twin;

pub use error::{FmiError, FmiResult};
