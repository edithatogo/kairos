use std::env::{self, consts};
use std::fs;
use std::path::{Path, PathBuf};

use kairo_ecs_fmi::export::model_description::{
    Causality, ModelDescription, ScalarVariable, Variability, VariableType,
};
use kairo_ecs_fmi::export::packager::{package_fmu_archive, write_unpacked_fmu};
use kairo_ecs_fmi::import::dynamic_loader::LoadedFmuLibrary;
use kairo_ecs_fmi::import::fmu_loader::{current_fmi_platform, FmuArchive};
use kairo_ecs_fmi::import::instance::FmuInstance;
use kairo_ecs_model::{DIGITS, MODEL_GUID, STEPS};

const MODEL_IDENTIFIER: &str = "kairo_ecs_model";
const STEP: f64 = 0.25;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<PathBuf> = env::args_os().skip(1).map(PathBuf::from).collect();
    let [output] = args.as_slice() else {
        return Err("usage: kairo-fmi-scheduled-counter <new-output-directory>".into());
    };
    let binary = env::current_exe()?.with_file_name(format!(
        "{}{MODEL_IDENTIFIER}{}",
        consts::DLL_PREFIX,
        consts::DLL_SUFFIX
    ));
    fs::create_dir(output)?;

    let mut description = ModelDescription::new("Kairos scheduled counter", MODEL_GUID);
    for (name, reference) in [("digits", DIGITS), ("steps", STEPS)] {
        description = description.with_variable(ScalarVariable {
            name: name.into(),
            value_reference: reference,
            causality: Causality::Output,
            variability: Variability::Discrete,
            variable_type: VariableType::Real,
        });
    }
    let layout = write_unpacked_fmu(output.join("unpacked"), &description)?;

    // The native Rust component owns its allocations. Declare that capability
    // in the example descriptor before packaging it for an external importer.
    let xml = fs::read_to_string(&layout.model_description)?;
    let (before, after) = xml
        .split_once("<CoSimulation modelIdentifier=\"kairo_ecs_model\"/>")
        .ok_or("expected the generated co-simulation declaration")?;
    fs::write(
        &layout.model_description,
        format!(
            "{before}<CoSimulation modelIdentifier=\"{MODEL_IDENTIFIER}\" \
             canNotUseMemoryManagementFunctions=\"true\"/>{after}"
        ),
    )?;

    let platform = if consts::ARCH == "aarch64" {
        let os = if cfg!(target_os = "macos") {
            "darwin"
        } else {
            consts::OS
        };
        format!("aarch64-{os}")
    } else {
        current_fmi_platform()
    };
    let directory = layout.root.join("binaries").join(platform);
    fs::create_dir_all(&directory)?;
    fs::copy(
        binary,
        directory
            .join(MODEL_IDENTIFIER)
            .with_extension(consts::DLL_EXTENSION),
    )?;

    let archive = output.join("KairosCounter.fmu");
    package_fmu_archive(&layout.root, &archive)?;
    verify(&archive, &output.join("readback"))
}

fn verify(archive: &Path, destination: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let layout = FmuArchive::new(archive).extract_to(destination)?;
    let library = LoadedFmuLibrary::load_from_path(layout.binary())?;
    // The counter has no resources.
    let mut instance = library.instantiate_checked("scheduled-counter", "")?;
    drop(library);
    instance.setup_experiment(None, 0.0, Some(3.0))?;
    instance.enter_initialization_mode()?;
    instance.exit_initialization_mode()?;

    println!("time,digits,steps");
    for sample in 0..=12 {
        let time = f64::from(sample) * STEP;
        if sample != 0 {
            instance.do_step(time - STEP, STEP)?;
        }
        let values = instance.get_real(&[DIGITS, STEPS])?;
        let expected = match sample / 4 {
            0 => [0.0, 0.0],
            1 => [47.0, 2.0],
            2 => [472.0, 3.0],
            _ => [4729.0, 4.0],
        };
        if values != expected {
            return Err(format!("sample {time} expected {expected:?}, got {values:?}").into());
        }
        println!("{time},{},{}", values[0], values[1]);
    }
    instance.terminate()?;
    Ok(())
}
