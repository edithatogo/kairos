use std::fs;
use std::path::{Component, Path, PathBuf};

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

    pub fn extract_to(&self, destination: impl AsRef<Path>) -> FmiResult<FmuLayout> {
        extract_stored_zip_entries(&self.path, destination.as_ref())?;
        FmuLayout::from_unpacked_dir(destination.as_ref())
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

fn extract_stored_zip_entries(path: &Path, destination: &Path) -> FmiResult<()> {
    let bytes =
        fs::read(path).map_err(|error| io_error("read FMU archive", path.to_path_buf(), error))?;
    fs::create_dir_all(destination)
        .map_err(|error| io_error("create FMU extraction directory", destination.into(), error))?;

    let mut offset = 0usize;
    let mut extracted_entries = 0usize;
    while offset < bytes.len() {
        let signature = read_u32(&bytes, offset).ok_or_else(|| {
            validation_error(
                "FMU archive",
                format!("truncated header in {}", path.display()),
            )
        })?;
        if signature == 0x0201_4b50 || signature == 0x0605_4b50 {
            break;
        }
        if signature != 0x0403_4b50 {
            return Err(validation_error(
                "FMU archive",
                format!("unexpected ZIP signature 0x{signature:08x} at byte {offset}"),
            ));
        }

        let flags = read_u16(&bytes, offset + 6).ok_or_else(|| {
            validation_error(
                "FMU archive",
                format!("truncated flags in {}", path.display()),
            )
        })?;
        if flags & 0x0008 != 0 {
            return Err(FmiError::UnsupportedArchiveExtraction {
                path: path.to_path_buf(),
            });
        }

        let method = read_u16(&bytes, offset + 8).ok_or_else(|| {
            validation_error(
                "FMU archive",
                format!("truncated method in {}", path.display()),
            )
        })?;
        if method != 0 {
            return Err(FmiError::UnsupportedArchiveCompression {
                path: path.to_path_buf(),
                method,
            });
        }

        let compressed_len = read_u32(&bytes, offset + 18).ok_or_else(|| {
            validation_error(
                "FMU archive",
                format!("truncated size in {}", path.display()),
            )
        })? as usize;
        let uncompressed_len = read_u32(&bytes, offset + 22).ok_or_else(|| {
            validation_error(
                "FMU archive",
                format!("truncated size in {}", path.display()),
            )
        })? as usize;
        if compressed_len != uncompressed_len {
            return Err(validation_error(
                "FMU archive",
                "stored entry has mismatched compressed and uncompressed sizes",
            ));
        }

        let name_len = read_u16(&bytes, offset + 26).ok_or_else(|| {
            validation_error(
                "FMU archive",
                format!("truncated name length in {}", path.display()),
            )
        })? as usize;
        let extra_len = read_u16(&bytes, offset + 28).ok_or_else(|| {
            validation_error(
                "FMU archive",
                format!("truncated extra length in {}", path.display()),
            )
        })? as usize;
        let name_start = offset + 30;
        let name_end = name_start
            .checked_add(name_len)
            .ok_or_else(|| validation_error("FMU archive", "archive entry name length overflow"))?;
        let data_start = name_end
            .checked_add(extra_len)
            .ok_or_else(|| validation_error("FMU archive", "archive extra length overflow"))?;
        let data_end = data_start
            .checked_add(compressed_len)
            .ok_or_else(|| validation_error("FMU archive", "archive entry size overflow"))?;
        if data_end > bytes.len() {
            return Err(validation_error(
                "FMU archive",
                format!("truncated entry data in {}", path.display()),
            ));
        }

        let entry = std::str::from_utf8(&bytes[name_start..name_end])
            .map_err(|error| validation_error("FMU archive", error.to_string()))?;
        let relative_path = safe_archive_entry(path, entry)?;
        let target = destination.join(relative_path);
        if entry.ends_with('/') {
            fs::create_dir_all(&target)
                .map_err(|error| io_error("create FMU archive directory", target, error))?;
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|error| {
                    io_error(
                        "create FMU archive parent directory",
                        parent.to_path_buf(),
                        error,
                    )
                })?;
            }
            fs::write(&target, &bytes[data_start..data_end])
                .map_err(|error| io_error("write FMU archive entry", target, error))?;
        }
        extracted_entries += 1;
        offset = data_end;
    }

    if extracted_entries == 0 {
        return Err(validation_error(
            "FMU archive",
            "archive contains no local entries",
        ));
    }
    Ok(())
}

fn safe_archive_entry(archive: &Path, entry: &str) -> FmiResult<PathBuf> {
    let path = Path::new(entry);
    if path.is_absolute() {
        return Err(FmiError::UnsafeArchiveEntry {
            path: archive.to_path_buf(),
            entry: entry.to_string(),
        });
    }

    let mut safe = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => safe.push(part),
            Component::CurDir => {}
            _ => {
                return Err(FmiError::UnsafeArchiveEntry {
                    path: archive.to_path_buf(),
                    entry: entry.to_string(),
                });
            }
        }
    }

    if safe.as_os_str().is_empty() {
        return Err(FmiError::UnsafeArchiveEntry {
            path: archive.to_path_buf(),
            entry: entry.to_string(),
        });
    }
    Ok(safe)
}

fn read_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    let slice = bytes.get(offset..offset.checked_add(2)?)?;
    Some(u16::from_le_bytes([slice[0], slice[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    let slice = bytes.get(offset..offset.checked_add(4)?)?;
    Some(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
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
