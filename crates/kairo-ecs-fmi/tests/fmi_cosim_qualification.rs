#![cfg(all(feature = "fmi-runtime", feature = "fmi2"))]

use std::path::PathBuf;
use std::process::Command;

use kairo_ecs_fmi::import::dynamic_loader::LoadedFmuLibrary;
use kairo_ecs_fmi::import::instance::{Fmi2LifecycleState, FmuInstance};

fn compile_test_cdylib() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src_path = manifest_dir.join("src/mock_fmu.rs");
    let out_dir = std::env::temp_dir().join("kairo_fmi_test_build");
    let _ = std::fs::create_dir_all(&out_dir);

    #[cfg(target_os = "macos")]
    let lib_name = "libmock_fmu.dylib";
    #[cfg(target_os = "linux")]
    let lib_name = "libmock_fmu.so";
    #[cfg(target_os = "windows")]
    let lib_name = "mock_fmu.dll";

    let target_path = out_dir.join(lib_name);

    let status = Command::new("rustc")
        .arg("--crate-type=cdylib")
        .arg("-O")
        .arg(&src_path)
        .arg("-o")
        .arg(&target_path)
        .status()
        .expect("rustc invocation failed");

    assert!(status.success(), "failed to build test cdylib");
    target_path
}

#[test]
fn test_fmi_dynamic_loading_and_1000_steps() {
    let dylib_path = compile_test_cdylib();

    // 1. Dynamic symbol binding and loading gate: fmi-dynamic-loading
    let loaded_lib = LoadedFmuLibrary::load_from_path(&dylib_path)
        .expect("Failed to dynamically load FMU library");

    // 2. Lifecycle instantiate
    let mut instance = loaded_lib
        .instantiate_checked("test_plant_0", "resources/")
        .expect("Failed to instantiate FMU component");

    assert_eq!(instance.lifecycle_state(), Fmi2LifecycleState::Instantiated);

    // 3. Setup experiment
    instance
        .setup_experiment(Some(1e-4), 0.0, Some(100.0))
        .expect("setup experiment failed");

    // 4. Enter initialization mode
    instance
        .enter_initialization_mode()
        .expect("enter initialization mode failed");
    assert_eq!(
        instance.lifecycle_state(),
        Fmi2LifecycleState::InitializationMode
    );

    // 5. Exit initialization mode
    instance
        .exit_initialization_mode()
        .expect("exit initialization mode failed");
    assert_eq!(instance.lifecycle_state(), Fmi2LifecycleState::StepMode);

    // 6. Run 1,000 co-simulation steps (fmi2-cosim-1000-step gate)
    let step_size = 0.01;
    for step in 0..1000 {
        let current_time = (step as f64) * step_size;
        instance
            .do_step(current_time, step_size)
            .expect("do_step failed");

        let vr = [1u32];
        let values = instance.get_real(&vr).expect("get_real failed");
        assert_eq!(values.len(), 1);
        assert!((values[0] - (42.0 + (step as f64) + 1.0)).abs() < 1e-6);
    }

    // 7. Terminate & free
    instance.terminate().expect("terminate failed");
    assert_eq!(instance.lifecycle_state(), Fmi2LifecycleState::Terminated);

    println!("Qualified 1,000 co-simulation steps on dynamically loaded native FMU!");
}
