#![forbid(unsafe_code)]

//! Diplomat bridge crate skeleton.
//!
//! This crate anchors the future generated Diplomat output to the stable FFI
//! surface so the owner path exists before code generation lands.

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
