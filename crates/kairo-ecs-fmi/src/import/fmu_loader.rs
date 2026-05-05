use std::fs;
use std::path::{Path, PathBuf};

use crate::{error::io_error, FmiError, FmiResult};

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
        let model_description = root.join("modelDescription.xml");
        if !model_description.is_file() {
            return Err(FmiError::MissingModelDescription { root });
        }

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
        fs::write(root.join("modelDescription.xml"), "<fmiModelDescription />").expect("xml");
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
}
