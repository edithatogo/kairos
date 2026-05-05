use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{io_error, validation_error};
use crate::export::model_description::ModelDescription;
use crate::FmiResult;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FmuPackageLayout {
    pub root: PathBuf,
    pub model_description: PathBuf,
    pub resources_dir: PathBuf,
    pub sources_dir: PathBuf,
}

pub fn write_unpacked_fmu(
    root: impl AsRef<Path>,
    model_description: &ModelDescription,
) -> FmiResult<FmuPackageLayout> {
    let root = root.as_ref().to_path_buf();
    model_description.validate()?;

    let resources_dir = root.join("resources");
    let sources_dir = root.join("sources");
    fs::create_dir_all(&resources_dir)
        .map_err(|error| io_error("create resources directory", resources_dir.clone(), error))?;
    fs::create_dir_all(&sources_dir)
        .map_err(|error| io_error("create sources directory", sources_dir.clone(), error))?;

    let model_description_path = root.join("modelDescription.xml");
    fs::write(
        &model_description_path,
        model_description.to_fmi2_xml().as_bytes(),
    )
    .map_err(|error| {
        io_error(
            "write modelDescription.xml",
            model_description_path.clone(),
            error,
        )
    })?;

    Ok(FmuPackageLayout {
        root,
        model_description: model_description_path,
        resources_dir,
        sources_dir,
    })
}

pub fn validate_unpacked_export_layout(root: impl AsRef<Path>) -> FmiResult<FmuPackageLayout> {
    let root = root.as_ref().to_path_buf();
    if !root.is_dir() {
        return Err(validation_error(
            "FMU export layout",
            format!("root is not a directory: {}", root.display()),
        ));
    }

    let model_description = root.join("modelDescription.xml");
    if !model_description.is_file() {
        return Err(validation_error(
            "FMU export layout",
            format!("missing {}", model_description.display()),
        ));
    }

    let resources_dir = root.join("resources");
    if !resources_dir.is_dir() {
        return Err(validation_error(
            "FMU export layout",
            format!("missing resources directory at {}", resources_dir.display()),
        ));
    }

    let sources_dir = root.join("sources");
    if !sources_dir.is_dir() {
        return Err(validation_error(
            "FMU export layout",
            format!("missing sources directory at {}", sources_dir.display()),
        ));
    }

    Ok(FmuPackageLayout {
        root,
        model_description,
        resources_dir,
        sources_dir,
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use crate::export::model_description::ScalarVariable;

    #[test]
    fn validates_written_unpacked_export_layout() {
        let root = std::env::temp_dir().join(format!(
            "kairo-fmu-export-layout-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let model_description = ModelDescription::new("oscillator", "{kairo-test}")
            .with_variable(ScalarVariable::real_output("position", 1));

        write_unpacked_fmu(&root, &model_description).expect("write export layout");
        let layout = validate_unpacked_export_layout(&root).expect("validate export layout");

        assert_eq!(layout.root, root);
        assert!(layout.model_description.is_file());
        assert!(layout.resources_dir.is_dir());
        assert!(layout.sources_dir.is_dir());

        fs::remove_dir_all(root).expect("cleanup");
    }
}
