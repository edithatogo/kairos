#![cfg(any(feature = "fmi2", feature = "fmi3"))]

use kairo_ecs_fmi::import::fmu_loader::{FmuArchive, FmuLayout};
use kairo_ecs_fmi::FmiError;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const STEP: f64 = 0.01;
const GRAVITY: f64 = 9.81;
const RESTITUTION: f64 = 0.7;
const HEIGHT_TOLERANCE: f64 = 0.015;
const VELOCITY_TOLERANCE: f64 = 0.03;
const VARIABLES: [u32; 2] = [1, 3];

struct ReferenceArchive(FmuLayout);

impl ReferenceArchive {
    fn bouncing_ball(version: &str) -> Self {
        let verified = std::env::var_os("KAIRO_REFERENCE_FMUS_DIR")
            .expect("point KAIRO_REFERENCE_FMUS_DIR at the verified Reference-FMUs release");
        let fmu = PathBuf::from(verified)
            .join(version)
            .join("BouncingBall.fmu");
        Self::extract(fmu, version)
    }

    fn extract(fmu: PathBuf, label: &str) -> Self {
        Self(
            FmuArchive::new(fmu)
                .extract_to(scratch_path(label))
                .unwrap(),
        )
    }

    fn repackage(&self, label: &str) -> Self {
        use kairo_ecs_fmi::export::packager::package_fmu_archive;

        let path = self.0.root().join("roundtrip.fmu");
        package_fmu_archive(self.0.root(), &path).unwrap();
        let packaged = Self::extract(path, label);
        for (original, copy) in [
            (self.0.model_description(), packaged.0.model_description()),
            (self.0.binary(), packaged.0.binary()),
        ] {
            assert_eq!(
                std::fs::read(original).unwrap(),
                std::fs::read(copy).unwrap()
            );
        }
        packaged
    }

    fn resources(&self) -> String {
        let resources = self.0.root().join("resources");
        std::fs::create_dir_all(&resources).unwrap();
        format!("{}/", resources.display())
    }
}

impl Drop for ReferenceArchive {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(self.0.root());
    }
}

fn scratch_path(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "kairos-reference-fmu-{label}-{}-{nonce}",
        std::process::id()
    ))
}

#[cfg(feature = "fmi2")]
#[test]
fn packaged_manifest_selects_the_model_and_rejects_asynchronous_fmi2() {
    use kairo_ecs_fmi::import::dynamic_loader::LoadedFmuLibrary;
    use kairo_ecs_fmi::import::fmu_loader::current_fmi_platform;
    use std::fs;

    let root = scratch_path("manifest-admission");
    let directory = root.join("binaries").join(current_fmi_platform());
    fs::create_dir_all(&directory).unwrap();
    let binary = directory
        .join("plant")
        .with_extension(std::env::consts::DLL_EXTENSION);
    let other = directory
        .join("dependency")
        .with_extension(std::env::consts::DLL_EXTENSION);
    for file in [&binary, &other] {
        fs::write(file, b"layout fixture").unwrap();
    }
    let description = root.join("modelDescription.xml");
    fs::write(
        &description,
        r#"<fmiModelDescription fmiVersion="2.0" guid="plant-guid">
           <CoSimulation modelIdentifier="plant" canRunAsynchronuously="true"/>
           </fmiModelDescription>"#,
    )
    .unwrap();
    assert_eq!(
        FmuLayout::from_unpacked_dir(&root).unwrap().binary(),
        binary
    );
    assert!(matches!(
        LoadedFmuLibrary::load_from_path(&other),
        Err(FmiError::Validation { .. })
    ));
    assert!(matches!(
        LoadedFmuLibrary::load_from_path(&binary),
        Err(FmiError::UnsupportedCapability(_))
    ));
    fs::remove_dir_all(root).unwrap();
}

fn analytic_sample(t: f64) -> [f64; 2] {
    let impact_time = (2.0 / GRAVITY).sqrt();
    if t < impact_time {
        [1.0 - 0.5 * GRAVITY * t * t, -GRAVITY * t]
    } else {
        let elapsed = t - impact_time;
        let rebound_velocity = RESTITUTION * GRAVITY * impact_time;
        [
            (rebound_velocity - 0.5 * GRAVITY * elapsed) * elapsed,
            rebound_velocity - GRAVITY * elapsed,
        ]
    }
}

macro_rules! check_bounce_and_replay {
    ($model:ident, $type:ty, $getter:ident) => {{
        let trace = |model: &mut $type, start, end| {
            let mut samples = Vec::new();
            for tick in start..end {
                model.do_step(f64::from(tick) * STEP, STEP).unwrap();
                let t = f64::from(tick + 1) * STEP;
                let actual: [f64; 2] = model.$getter(&VARIABLES).unwrap().try_into().unwrap();
                let expected = analytic_sample(t);
                assert!(
                    (actual[0] - expected[0]).abs() <= HEIGHT_TOLERANCE,
                    "height at t={t} was {}",
                    actual[0]
                );
                // The fixed 1 ms solver localizes the discontinuous velocity on
                // its own grid. Test velocity on either side of that window.
                if (t - (2.0 / GRAVITY).sqrt()).abs() > 0.002 {
                    assert!(
                        (actual[1] - expected[1]).abs() <= VELOCITY_TOLERANCE,
                        "velocity at t={t} was {}",
                        actual[1]
                    );
                }
                samples.push(actual.map(f64::to_bits));
            }
            samples
        };

        assert!(matches!(
            $model.restore_state(),
            Err(FmiError::InvalidInput(_))
        ));
        let mut trajectory = trace(&mut $model, 0, 40);
        $model.save_state().unwrap();
        let first = trace(&mut $model, 40, 60);
        assert!(
            f64::from_bits(first.last().unwrap()[1]) > 0.0,
            "the ball must rebound"
        );
        $model.restore_state().unwrap();
        assert_eq!(trace(&mut $model, 40, 60), first);
        $model.discard_state().unwrap();
        assert!(matches!(
            $model.restore_state(),
            Err(FmiError::InvalidInput(_))
        ));
        $model.save_state().unwrap();
        $model.terminate().unwrap();
        trajectory.extend(first);
        trajectory
    }};
}

#[cfg(feature = "fmi2")]
#[test]
#[ignore = "requires the separately verified Modelica Association binary FMUs"]
fn fmi2_bouncing_ball_replays_a_native_impact() {
    let original = ReferenceArchive::bouncing_ball("2.0");
    let repackaged = original.repackage("2.0-repackaged");
    assert_eq!(run_fmi2_bounce(&original), run_fmi2_bounce(&repackaged));
}

#[cfg(feature = "fmi2")]
fn run_fmi2_bounce(archive: &ReferenceArchive) -> Vec<[u64; 2]> {
    use kairo_ecs_fmi::import::dynamic_loader::LoadedFmuLibrary;
    use kairo_ecs_fmi::import::instance::{Fmi2CoSimulationInstance, FmuInstance};

    let library = LoadedFmuLibrary::load_from_path(archive.0.binary()).unwrap();
    let mut instance = library
        .instantiate_checked("bounce", &format!("file://{}", archive.resources()))
        .unwrap();
    drop(library);
    instance.setup_experiment(None, 0.0, Some(0.8)).unwrap();
    instance.enter_initialization_mode().unwrap();
    instance.exit_initialization_mode().unwrap();
    check_bounce_and_replay!(instance, Fmi2CoSimulationInstance, get_real)
}

#[cfg(feature = "fmi3")]
#[test]
#[ignore = "requires the separately verified Modelica Association binary FMUs"]
fn fmi3_bouncing_ball_replays_a_native_impact() {
    let original = ReferenceArchive::bouncing_ball("3.0");
    let repackaged = original.repackage("3.0-repackaged");
    assert_eq!(run_fmi3_bounce(&original), run_fmi3_bounce(&repackaged));
}

#[cfg(feature = "fmi3")]
fn run_fmi3_bounce(archive: &ReferenceArchive) -> Vec<[u64; 2]> {
    use kairo_ecs_fmi::import::dynamic_loader::LoadedFmi3Library;
    use kairo_ecs_fmi::import::fmi3::Fmi3CoSimulationInstance;

    let library = LoadedFmi3Library::load_from_path(archive.0.binary()).unwrap();
    let mut instance = library
        .instantiate_checked("bounce", &archive.resources())
        .unwrap();
    drop(library);
    instance
        .enter_initialization_mode(None, 0.0, Some(0.8))
        .unwrap();
    instance.exit_initialization_mode().unwrap();
    check_bounce_and_replay!(instance, Fmi3CoSimulationInstance, get_float64)
}

#[cfg(feature = "fmi2")]
#[test]
#[ignore = "requires the separately built OpenModelica co-simulation FMU"]
fn openmodelica_fmi2_executes_a_time_event_and_rejects_unavailable_state() {
    use kairo_ecs_fmi::export::model_description::ModelDescription;
    use kairo_ecs_fmi::import::dynamic_loader::LoadedFmuLibrary;
    use kairo_ecs_fmi::import::instance::FmuInstance;

    let path = std::env::var_os("KAIRO_OPENMODELICA_FMU")
        .expect("point KAIRO_OPENMODELICA_FMU at the independently built archive");
    let archive = ReferenceArchive::extract(PathBuf::from(path), "openmodelica");
    let xml = std::fs::read_to_string(archive.0.model_description()).unwrap();
    let description = ModelDescription::from_xml_str(&xml).unwrap();
    assert!(description.generation_tool.starts_with("OpenModelica"));
    let output = description
        .variables
        .iter()
        .find(|variable| variable.name == "x")
        .unwrap();

    let library = LoadedFmuLibrary::load_from_path(archive.0.binary()).unwrap();
    let mut instance = library
        .instantiate_checked("openmodelica", &format!("file://{}", archive.resources()))
        .unwrap();
    drop(library);
    instance.setup_experiment(None, 0.0, Some(1.0)).unwrap();
    instance.enter_initialization_mode().unwrap();
    instance.exit_initialization_mode().unwrap();
    assert!(matches!(
        instance.save_state(),
        Err(FmiError::UnsupportedCapability(_))
    ));
    for tick in 0..75 {
        instance.do_step(f64::from(tick) * STEP, STEP).unwrap();
        let t = f64::from(tick + 1) * STEP;
        if (t - 0.5).abs() > 0.011 {
            let expected = if t < 0.5 {
                (-t).exp()
            } else {
                2.0 * (0.5 - t).exp()
            };
            let observed = instance.get_real(&[output.value_reference]).unwrap()[0];
            assert!(
                (observed - expected).abs() <= 0.012,
                "OpenModelica x({t}) was {observed}"
            );
        }
    }
    instance.terminate().unwrap();
}
