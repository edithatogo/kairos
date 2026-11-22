//! Canonical trusted-owned imports; implementation lives in the shared audited
//! Arrow FFI boundary. Existing native-data consumer paths remain available.
pub use kairo_ecs_arrow_ffi::native_tables::{import_batch, import_stream, TableStreamReader};
