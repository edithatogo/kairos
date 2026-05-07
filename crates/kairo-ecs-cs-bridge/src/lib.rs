//! C# P/Invoke bridge for KairoECS
//!
//! Re-exports the FFI functions in a form C# P/Invoke can consume.
//! This crate exists so C# can load the single .dll/.so/.dylib and
//! get both the core ABI and the C#-specific convenience layer.

pub use kairo_ecs_ffi::*;
