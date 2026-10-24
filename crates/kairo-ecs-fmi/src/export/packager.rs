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
    model_description.validate_generated_fmi2_xml()?;

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

pub fn package_fmu_archive(
    unpacked_root: impl AsRef<Path>,
    output_fmu_path: impl AsRef<Path>,
) -> FmiResult<()> {
    let unpacked_root = unpacked_root.as_ref();
    let output_fmu_path = output_fmu_path.as_ref();

    if !unpacked_root.is_dir() {
        return Err(validation_error(
            "FMU packager",
            format!(
                "unpacked root is not a directory: {}",
                unpacked_root.display()
            ),
        ));
    }

    let model_description = unpacked_root.join("modelDescription.xml");
    if !model_description.is_file() {
        return Err(validation_error(
            "FMU packager",
            format!(
                "unpacked root {} is missing modelDescription.xml",
                unpacked_root.display()
            ),
        ));
    }

    if let Some(parent) = output_fmu_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|error| {
                io_error(
                    "create parent directory for FMU package",
                    parent.to_path_buf(),
                    error,
                )
            })?;
        }
    }

    let mut entries = Vec::new();
    collect_directory_entries(unpacked_root, unpacked_root, &mut entries)?;
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let packaging_result = (|| -> FmiResult<()> {
        let file = fs::File::create(output_fmu_path).map_err(|error| {
            io_error(
                "create output FMU file",
                output_fmu_path.to_path_buf(),
                error,
            )
        })?;

        let mut zip_writer = zip::ZipWriter::new(file);

        for (rel_path_str, source_path, is_dir) in entries {
            if is_dir {
                let options = zip::write::SimpleFileOptions::default();
                zip_writer
                    .add_directory(&rel_path_str, options)
                    .map_err(|error| validation_error("FMU packager", error.to_string()))?;
            } else {
                let options = zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Deflated);
                zip_writer
                    .start_file(&rel_path_str, options)
                    .map_err(|error| validation_error("FMU packager", error.to_string()))?;

                let mut in_file = fs::File::open(&source_path).map_err(|error| {
                    io_error("read FMU source file for packaging", source_path, error)
                })?;
                std::io::copy(&mut in_file, &mut zip_writer).map_err(|error| {
                    io_error(
                        "write FMU file into archive",
                        output_fmu_path.to_path_buf(),
                        error,
                    )
                })?;
            }
        }

        zip_writer
            .finish()
            .map_err(|error| validation_error("FMU packager", error.to_string()))?;
        Ok(())
    })();

    if packaging_result.is_err() {
        let _ = fs::remove_file(output_fmu_path);
    }

    packaging_result
}

fn collect_directory_entries(
    root: &Path,
    current: &Path,
    out: &mut Vec<(String, PathBuf, bool)>,
) -> FmiResult<()> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(current)
        .map_err(|error| io_error("read directory for packaging", current.to_path_buf(), error))?
    {
        let entry = entry.map_err(|error| {
            io_error(
                "read directory entry for packaging",
                current.to_path_buf(),
                error,
            )
        })?;
        entries.push(entry.path());
    }
    entries.sort();

    for path in entries {
        let rel_path = path
            .strip_prefix(root)
            .map_err(|error| validation_error("FMU packager", error.to_string()))?;
        let rel_components: Vec<_> = rel_path
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect();
        let rel_str = rel_components.join("/");

        if path.is_dir() {
            out.push((format!("{rel_str}/"), path.clone(), true));
            collect_directory_entries(root, &path, out)?;
        } else if path.is_file() {
            out.push((rel_str, path, false));
        }
    }
    Ok(())
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
