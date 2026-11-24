use std::env::{self, consts};
use std::fs;
use std::path::{Path, PathBuf};

use kairo_ecs_fmi::export::model_description::{
    Causality, ModelDescription, ScalarVariable, Variability, VariableType,
};
use kairo_ecs_fmi::export::packager::{
    package_fmu_archive, write_unpacked_fmi3_fmu, write_unpacked_fmu,
};
use kairo_ecs_fmi::import::dynamic_loader::{LoadedFmi3Library, LoadedFmuLibrary};
use kairo_ecs_fmi::import::fmu_loader::{current_fmi_platform, FmuArchive};
use kairo_ecs_fmi::import::instance::FmuInstance;
use kairo_ecs_model::{DIGITS, MODEL_GUID, STEPS, TIME};

const MODEL_IDENTIFIER: &str = "kairo_ecs_model";
const STEP: f64 = 0.25;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<PathBuf> = env::args_os().skip(1).map(PathBuf::from).collect();
    let (fmi3, output) = match args.as_slice() {
        [output] if output.as_os_str() != "--fmi3" => (false, output),
        [flag, output] if flag.as_os_str() == "--fmi3" => (true, output),
        _ => {
            return Err("usage: kairo-fmi-scheduled-counter [--fmi3] <new-output-directory>".into())
        }
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
    let layout = if fmi3 {
        write_unpacked_fmi3_fmu(output.join("unpacked"), &description, TIME, &[])?
    } else {
        let layout = write_unpacked_fmu(output.join("unpacked"), &description)?;
        // The FMI 2 component owns its allocations in Rust.
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
        layout
    };

    let platform = if fmi3 || consts::ARCH == "aarch64" {
        let os = if cfg!(target_os = "macos") {
            "darwin"
        } else {
            consts::OS
        };
        let arch = if consts::ARCH == "arm" {
            "aarch32"
        } else {
            consts::ARCH
        };
        format!("{arch}-{os}")
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
    verify(&archive, &output.join("readback"), fmi3)
}

fn verify(
    archive: &Path,
    destination: &Path,
    fmi3: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let layout = FmuArchive::new(archive).extract_to(destination)?;
    if fmi3 {
        let library = LoadedFmi3Library::load_from_path(layout.binary())?;
        let mut resources = layout
            .root()
            .join("resources")
            .canonicalize()?
            .into_os_string();
        resources.push(std::path::MAIN_SEPARATOR_STR);
        let resources = resources
            .to_str()
            .ok_or("FMI 3 resource path must be UTF-8")?;
        let mut instance = library.instantiate_checked("scheduled-counter", resources)?;
        drop(library);
        instance.enter_initialization_mode(None, 0.0, Some(3.0))?;
        instance.exit_initialization_mode()?;
        verify_samples(|time| {
            if time > 0.0 {
                instance.do_step(time - STEP, STEP)?;
            }
            let values = instance.get_float64(&[TIME, DIGITS, STEPS])?;
            if values[0] != time {
                return Err(format!("expected clock {time}, got {}", values[0]).into());
            }
            Ok([values[1], values[2]])
        })?;
        instance.terminate()?;
    } else {
        let library = LoadedFmuLibrary::load_from_path(layout.binary())?;
        let mut instance = library.instantiate_checked("scheduled-counter", "")?;
        drop(library);
        instance.setup_experiment(None, 0.0, Some(3.0))?;
        instance.enter_initialization_mode()?;
        instance.exit_initialization_mode()?;
        verify_samples(|time| {
            if time > 0.0 {
                instance.do_step(time - STEP, STEP)?;
            }
            let values = instance.get_real(&[DIGITS, STEPS])?;
            Ok([values[0], values[1]])
        })?;
        instance.terminate()?;
    }
    Ok(())
}

fn verify_samples(
    mut sample_at: impl FnMut(f64) -> Result<[f64; 2], Box<dyn std::error::Error>>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("time,digits,steps");
    for sample in 0..=12 {
        let time = f64::from(sample) * STEP;
        let values = sample_at(time)?;
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
    Ok(())
}
