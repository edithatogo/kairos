🎯 **What:**
Added unit tests for the `is_webgpu_available` function in `crates/kairo-ecs-webgpu/src/adapter.rs` to ensure it correctly evaluates the `detect_adapter()` result.

📊 **Coverage:**
- Added the `Available` variant to `AdapterStatus`.
- Refactored `is_webgpu_available()` to check for `AdapterStatus::Available`.
- Made `detect_adapter()` mockable during tests using a thread-local `RefCell`.
- Added tests verifying `is_webgpu_available()` returns true when the adapter is available and false when it's not (unavailable, detected but not available, or not configured).

✨ **Result:**
Increased test coverage for `is_webgpu_available` by capturing various adapter detection edge cases, improving the reliability of WebGPU availability checks.
