use std::fs;
use std::path::{Path, PathBuf};

use crate::error::io_error;
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
