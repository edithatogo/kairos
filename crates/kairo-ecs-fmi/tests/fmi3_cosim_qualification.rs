#![cfg(all(feature = "fmi-runtime", feature = "fmi3"))]

use std::path::PathBuf;
use std::process::Command;

use kairo_ecs_fmi::import::dynamic_loader::LoadedFmi3Library;
use kairo_ecs_fmi::import::fmi3::Fmi3LifecycleState;
use kairo_ecs_fmi::FmiError;

fn compile_test_cdylib_fmi3() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src_path = manifest_dir.join("src/mock_fmu3.rs");
    let out_dir = std::env::temp_dir().join("kairo_fmi3_test_build");
    let _ = std::fs::create_dir_all(&out_dir);

    #[cfg(target_os = "macos")]
    let lib_name = "libmock_fmu3.dylib";
    #[cfg(target_os = "linux")]
    let lib_name = "libmock_fmu3.so";
    #[cfg(target_os = "windows")]
    let lib_name = "mock_fmu3.dll";

    let target_path = out_dir.join(lib_name);

    let status = Command::new("rustc")
        .arg("--crate-type=cdylib")
        .arg("-O")
        .arg(&src_path)
        .arg("-o")
        .arg(&target_path)
        .status()
        .expect("rustc invocation failed");

    assert!(status.success(), "failed to build FMI3 test cdylib");
    target_path
}

#[test]
fn test_fmi3_dynamic_loading_and_1000_steps() {
    let dylib_path = compile_test_cdylib_fmi3();

    // 1. Dynamic symbol binding and loading gate: fmi-dynamic-loading (FMI 3)
    let loaded_lib =
        LoadedFmi3Library::load_from_path(&dylib_path).expect("Failed to load FMI 3 library");

    // 2. Lifecycle instantiate
    let mut instance = loaded_lib
        .instantiate_with_token("test_plant_fmi3", "mock-fmi3", "resources/")
        .expect("Failed to instantiate FMI 3 component");
    drop(loaded_lib);

    assert_eq!(instance.lifecycle_state(), Fmi3LifecycleState::Instantiated);

    // 3. Enter initialization mode
    instance
        .enter_initialization_mode(Some(1e-4), 0.0, Some(100.0))
        .expect("enter initialization mode failed");
    assert_eq!(
        instance.lifecycle_state(),
        Fmi3LifecycleState::InitializationMode
    );

    // 4. Exit initialization mode
    instance
        .exit_initialization_mode()
        .expect("exit initialization mode failed");
    assert_eq!(instance.lifecycle_state(), Fmi3LifecycleState::StepMode);

    // 5. Run 1,000 co-simulation steps (fmi3-cosim-1000-step gate)
    let step_size = 0.01;
    for step in 0..1000 {
        let current_time = (step as f64) * step_size;
        instance
            .do_step(current_time, step_size)
            .expect("FMI 3 do_step failed");

        let vr = [1u32];
        let values = instance.get_float64(&vr).expect("get_float64 failed");
        assert_eq!(values.len(), 1);
        assert!((values[0] - (100.0 + (step as f64) + 1.0)).abs() < 1e-6);
    }
    assert!(matches!(
        instance.do_step(10.0, step_size),
        Err(FmiError::Validation { message, .. }) if message.contains("FMU reached")
    ));
    let one_ulp = f64::from_bits(11.0_f64.to_bits() + 1) - 11.0;
    assert!(matches!(
        instance.do_step(11.0, one_ulp),
        Err(FmiError::Validation { message, .. }) if message.contains("FMU reached")
    ));

    // 6. Terminate & free
    instance.terminate().expect("terminate failed");
    assert_eq!(instance.lifecycle_state(), Fmi3LifecycleState::Terminated);
    drop(instance);

    let library = LoadedFmi3Library::load_from_path(&dylib_path).unwrap();
    let counters = unsafe { libloading::Library::new(&dylib_path) }.unwrap();
    let calls = |symbol: &[u8]| {
        let counter: libloading::Symbol<unsafe extern "C" fn() -> i32> =
            unsafe { counters.get(symbol) }.unwrap();
        unsafe { counter() }
    };
    let terminations = calls(b"mockTerminateCount");
    let frees = calls(b"mockFreeCount");
    let start = || {
        let mut instance = library
            .instantiate_with_token("fault-control", "mock-fmi3", "resources/")
            .unwrap();
        instance.enter_initialization_mode(None, 0.0, None).unwrap();
        instance.exit_initialization_mode().unwrap();
        instance
    };
    let mut instance = start();
    assert!(matches!(
        instance.do_step(12.0, step_size),
        Err(FmiError::FmiStatus { status: 3, .. })
    ));
    assert_eq!(instance.lifecycle_state(), Fmi3LifecycleState::Terminated);
    assert!(instance.do_step(12.0, step_size).is_err());
    drop(instance);
    assert_eq!(calls(b"mockTerminateCount"), terminations);
    assert_eq!(calls(b"mockFreeCount"), frees + 1);

    let mut instance = start();
    let mut sibling = start();
    assert!(matches!(
        instance.do_step(13.0, step_size),
        Err(FmiError::FmiStatus { status: 4, .. })
    ));
    assert!(sibling.do_step(13.0, step_size).is_err());
    drop(instance);
    drop(sibling);
    assert_eq!(calls(b"mockTerminateCount"), terminations);
    assert_eq!(calls(b"mockFreeCount"), frees + 1);

    println!("Qualified 1,000 co-simulation steps on dynamically loaded native FMI 3 FMU!");
}
