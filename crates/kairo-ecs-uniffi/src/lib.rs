#![forbid(unsafe_code)]

//! UniFFI bridge crate skeleton.
//!
//! The generated wrapper surface will attach to the stable C ABI exposed by
//! `kairo-ecs-ffi`. This crate exists so the bridge roots are concrete and can
//! grow into generated output without changing the owner path.

pub use kairo_ecs_ffi::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reexports_bridge_version() {
        assert_eq!(
            kairo_ecs_ffi::kairo_ecs_ffi_version(),
            KAIRO_ECS_FFI_VERSION
        );
    }
}
