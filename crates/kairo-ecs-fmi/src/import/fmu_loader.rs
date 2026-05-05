use std::fs;
use std::path::{Path, PathBuf};

use crate::{
    error::{io_error, validation_error},
    FmiError, FmiResult,
};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FmuLayout {
    root: PathBuf,
    model_description: PathBuf,
    binary: PathBuf,
    platform: String,
}

impl FmuLayout {
    pub fn from_unpacked_dir(root: impl Into<PathBuf>) -> FmiResult<Self> {
        let root = root.into();
        validate_unpacked_fmu_layout(&root)?;

        let model_description = root.join("modelDescription.xml");
        let platform = current_fmi_platform();
        let binary = find_platform_binary(&root, &platform)?;
        Ok(Self {
            root,
            model_description,
            binary,
            platform,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn model_description(&self) -> &Path {
        &self.model_description
    }

    pub fn binary(&self) -> &Path {
        &self.binary
    }

    pub fn platform(&self) -> &str {
        &self.platform
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FmuLayoutReport {
    pub root: PathBuf,
    pub platform: String,
    pub model_description: PathBuf,
    pub binary_dir: PathBuf,
    pub binary_candidates: Vec<PathBuf>,
}

impl FmuLayoutReport {
    pub fn selected_binary(&self) -> Option<&Path> {
        self.binary_candidates.first().map(PathBuf::as_path)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FmuArchive {
    path: PathBuf,
}

impl FmuArchive {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn open_unpacked(root: impl Into<PathBuf>) -> FmiResult<FmuLayout> {
        FmuLayout::from_unpacked_dir(root)
    }

    pub fn extract_to(&self, _destination: impl AsRef<Path>) -> FmiResult<FmuLayout> {
        Err(FmiError::UnsupportedArchiveExtraction {
            path: self.path.clone(),
        })
    }
}

pub fn current_fmi_platform() -> String {
    let os = if cfg!(target_os = "windows") {
        "win"
    } else if cfg!(target_os = "macos") {
        "darwin"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        std::env::consts::OS
    };

    let arch = if cfg!(target_arch = "x86_64") {
        "64"
    } else if cfg!(target_arch = "x86") {
        "32"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        std::env::consts::ARCH
    };

    format!("{os}{arch}")
}

pub fn validate_unpacked_fmu_layout(root: impl AsRef<Path>) -> FmiResult<FmuLayoutReport> {
    let root = root.as_ref();
    if !root.is_dir() {
        return Err(validation_error(
            "FMU layout",
            format!("root is not a directory: {}", root.display()),
        ));
    }

    let model_description = root.join("modelDescription.xml");
    if !model_description.is_file() {
        return Err(FmiError::MissingModelDescription {
            root: root.to_path_buf(),
        });
    }
    validate_model_description_marker(&model_description)?;

    let platform = current_fmi_platform();
    let binary_dir = root.join("binaries").join(&platform);
    if !binary_dir.is_dir() {
        return Err(validation_error(
            "FMU layout",
            format!(
                "missing binary directory for platform {} at {}",
                platform,
                binary_dir.display()
            ),
        ));
    }

    let extension = shared_library_extension();
    let mut binary_candidates = Vec::new();
    for entry in fs::read_dir(&binary_dir)
        .map_err(|error| io_error("read FMU binary directory", binary_dir.clone(), error))?
    {
        let entry =
            entry.map_err(|error| io_error("read FMU binary entry", binary_dir.clone(), error))?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|value| value.to_str()) == Some(extension) {
            binary_candidates.push(path);
        }
    }
    binary_candidates.sort();

    if binary_candidates.is_empty() {
        return Err(FmiError::MissingBinary {
            platform,
            root: root.to_path_buf(),
        });
    }

    Ok(FmuLayoutReport {
        root: root.to_path_buf(),
        platform,
        model_description,
        binary_dir,
        binary_candidates,
    })
}

fn validate_model_description_marker(path: &Path) -> FmiResult<()> {
    let contents = fs::read_to_string(path)
        .map_err(|error| io_error("read modelDescription.xml", path.to_path_buf(), error))?;
    if !contents.contains("<fmiModelDescription") {
        return Err(validation_error(
            "FMU layout",
            format!(
                "{} does not contain an fmiModelDescription root",
                path.display()
            ),
        ));
    }
    if !contents.contains("fmiVersion=\"2.0\"") && !contents.contains("fmiVersion=\"3.0\"") {
        return Err(validation_error(
            "FMU layout",
            format!("{} does not declare FMI version 2.0 or 3.0", path.display()),
        ));
    }
    Ok(())
}

fn find_platform_binary(root: &Path, platform: &str) -> FmiResult<PathBuf> {
    let binary_dir = root.join("binaries").join(platform);
    let entries = fs::read_dir(&binary_dir)
        .map_err(|error| io_error("read FMU binary directory", binary_dir.clone(), error))?;

    let extension = shared_library_extension();
    for entry in entries {
        let entry =
            entry.map_err(|error| io_error("read FMU binary entry", binary_dir.clone(), error))?;
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) == Some(extension) {
            return Ok(path);
        }
    }

    Err(FmiError::MissingBinary {
        platform: platform.to_string(),
        root: root.to_path_buf(),
    })
}

fn shared_library_extension() -> &'static str {
    if cfg!(target_os = "windows") {
        "dll"
    } else if cfg!(target_os = "macos") {
        "dylib"
    } else {
        "so"
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn identifies_unpacked_fmu_layout() {
        let root = std::env::temp_dir().join(format!(
            "kairo-fmu-layout-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let platform = current_fmi_platform();
        let binary_dir = root.join("binaries").join(&platform);
        fs::create_dir_all(&binary_dir).expect("binary dir");
        fs::write(
            root.join("modelDescription.xml"),
            "<fmiModelDescription fmiVersion=\"2.0\" />",
        )
        .expect("xml");
        fs::write(
            binary_dir.join(format!("model.{}", shared_library_extension())),
            [],
        )
        .expect("binary");

        let layout = FmuLayout::from_unpacked_dir(&root).expect("layout");
        assert_eq!(layout.platform(), platform);
        assert!(layout.model_description().ends_with("modelDescription.xml"));

        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn reports_missing_platform_binary_directory() {
        let root = std::env::temp_dir().join(format!(
            "kairo-fmu-missing-binary-dir-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("root dir");
        fs::write(
            root.join("modelDescription.xml"),
            "<fmiModelDescription fmiVersion=\"2.0\" />",
        )
        .expect("xml");

        let error = validate_unpacked_fmu_layout(&root).expect_err("missing binary dir");
        assert!(error.to_string().contains("missing binary directory"));

        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn rejects_model_description_without_fmi_version() {
        let root = std::env::temp_dir().join(format!(
            "kairo-fmu-invalid-model-description-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let platform = current_fmi_platform();
        let binary_dir = root.join("binaries").join(&platform);
        fs::create_dir_all(&binary_dir).expect("binary dir");
        fs::write(root.join("modelDescription.xml"), "<fmiModelDescription />").expect("xml");
        fs::write(
            binary_dir.join(format!("model.{}", shared_library_extension())),
            [],
        )
        .expect("binary");

        let error = validate_unpacked_fmu_layout(&root).expect_err("invalid model description");
        assert!(error.to_string().contains("does not declare FMI version"));

        fs::remove_dir_all(root).expect("cleanup");
    }
}
