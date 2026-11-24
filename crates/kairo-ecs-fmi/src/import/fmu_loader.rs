use std::collections::HashSet;
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use quick_xml::events::Event;
use quick_xml::reader::Reader;
use quick_xml::XmlVersion;

use crate::{
    error::{io_error, validation_error},
    FmiError, FmiResult,
};

pub const MAX_ARCHIVE_FILES: usize = 10_000;
pub const MAX_EXPANDED_BYTES: u64 = 500 * 1024 * 1024; // 500 MB
pub const MAX_XML_BYTES: u64 = 50 * 1024 * 1024; // 50 MB
pub const MAX_XML_DEPTH: usize = 32;

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
        let report = validate_unpacked_fmu_layout(&root)?;
        let binary = report
            .selected_binary()
            .expect("validated layout has a binary")
            .to_path_buf();
        Ok(Self {
            root,
            model_description: report.model_description,
            binary,
            platform: report.platform,
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

    /// Extract into a new directory. Existing destinations are rejected so archive
    /// writes cannot follow files or symlinks supplied by a previous extraction.
    /// The caller must keep the destination's parent under its control while
    /// extraction runs.
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

/// Returns the FMI 3 binary directory name for the current target.
pub fn current_fmi3_platform() -> String {
    fmi3_platform(std::env::consts::ARCH, std::env::consts::OS)
}

fn fmi3_platform(arch: &str, os: &str) -> String {
    let arch = if arch == "arm" { "aarch32" } else { arch };
    let os = if os == "macos" { "darwin" } else { os };
    format!("{arch}-{os}")
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
    let description = read_model_description(&model_description)?;

    let standard = platform_for_version(&description.version);
    let platform = if root.join("binaries").join(&standard).is_dir() {
        standard
    } else {
        current_fmi_platform()
    };
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
    if let Some(model_identifier) = &description.model_identifier {
        if let Some(index) = binary_candidates.iter().position(|path| {
            path.file_stem().and_then(|name| name.to_str()) == Some(model_identifier)
        }) {
            binary_candidates[..=index].rotate_right(1);
        }
    }

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

#[derive(Default)]
struct FmuDescription {
    version: String,
    guid: Option<String>,
    instantiation_token: Option<String>,
    model_identifier: Option<String>,
    can_get_and_set_state: bool,
    can_run_asynchronously: bool,
}

#[cfg(any(feature = "fmi2", feature = "fmi3"))]
pub(crate) fn co_simulation_identity(
    binary: &Path,
    version: &str,
) -> FmiResult<Option<(String, bool)>> {
    let Some(binaries) = binary
        .parent()
        .and_then(Path::parent)
        .filter(|parent| parent.file_name().is_some_and(|name| name == "binaries"))
    else {
        return Ok(None);
    };
    let root = binaries.parent().expect("binaries directory has a parent");
    let description = read_model_description(&root.join("modelDescription.xml"))?;
    if description.version != version
        || description.model_identifier.as_deref() != binary.file_stem().and_then(|s| s.to_str())
        || description.model_identifier.is_none()
    {
        return Err(validation_error(
            "FMU layout",
            format!(
                "modelDescription.xml does not declare {version} CoSimulation for {}",
                binary.display()
            ),
        ));
    }
    let token = if version == "2.0" {
        if description.can_run_asynchronously {
            return Err(FmiError::UnsupportedCapability(
                "asynchronous FMI 2 co-simulation",
            ));
        }
        description.guid
    } else {
        description.instantiation_token
    };
    let token = token
        .filter(|token| !token.trim().is_empty())
        .ok_or_else(|| {
            validation_error(
                "FMU layout",
                format!("missing FMI {version} instance identity"),
            )
        })?;
    Ok(Some((token, description.can_get_and_set_state)))
}

fn platform_for_version(version: &str) -> String {
    if version == "2.0" && std::env::consts::ARCH != "aarch64" {
        return current_fmi_platform();
    }
    current_fmi3_platform()
}

fn read_model_description(path: &Path) -> FmiResult<FmuDescription> {
    let metadata = fs::metadata(path).map_err(|error| {
        io_error(
            "read modelDescription.xml metadata",
            path.to_path_buf(),
            error,
        )
    })?;
    if metadata.len() > MAX_XML_BYTES {
        return Err(validation_error(
            "FMU layout",
            format!(
                "{} exceeds maximum allowed size ({} bytes > {} bytes)",
                path.display(),
                metadata.len(),
                MAX_XML_BYTES
            ),
        ));
    }

    let contents = fs::read_to_string(path)
        .map_err(|error| io_error("read modelDescription.xml", path.to_path_buf(), error))?;
    let mut reader = Reader::from_str(&contents);
    reader.config_mut().trim_text(true);

    let mut depth = 0_usize;
    let mut found_root = false;
    let mut root_closed = false;
    let mut found_co_simulation = false;
    let mut description = FmuDescription::default();

    loop {
        let (element, open) = match reader.read_event() {
            Ok(Event::Start(element)) => (element, true),
            Ok(Event::Empty(element)) => (element, false),
            Ok(Event::End(e)) => {
                depth = depth.checked_sub(1).ok_or_else(|| {
                    validation_error(
                        "FMU layout",
                        format!("unmatched XML close in {}", path.display()),
                    )
                })?;
                root_closed = depth == 0 && e.local_name().as_ref() == "fmiModelDescription";
                continue;
            }
            Ok(Event::Eof) => break,
            Ok(Event::Text(t)) if (!found_root || root_closed) && !t.as_ref().trim().is_empty() => {
                return Err(validation_error(
                    "FMU layout",
                    format!("unexpected text outside root element in {}", path.display()),
                ));
            }
            Ok(_) => continue,
            Err(err) => {
                return Err(validation_error(
                    "FMU layout",
                    format!("malformed XML in {}: {err}", path.display()),
                ));
            }
        };

        if root_closed {
            return Err(validation_error(
                "FMU layout",
                format!(
                    "unexpected element <{}> after root element closed in {}",
                    element.local_name().as_ref(),
                    path.display()
                ),
            ));
        }
        let co_simulation = depth == 1 && element.local_name().as_ref() == "CoSimulation";
        if !found_root {
            if element.local_name().as_ref() != "fmiModelDescription" {
                return Err(validation_error(
                    "FMU layout",
                    format!(
                        "{} does not contain an fmiModelDescription root",
                        path.display()
                    ),
                ));
            }
        } else if co_simulation && found_co_simulation {
            return Err(validation_error(
                "FMU layout",
                format!("multiple CoSimulation declarations in {}", path.display()),
            ));
        }
        if !found_root || co_simulation {
            for attribute in element.attributes() {
                let attribute = attribute.map_err(|err| {
                    validation_error(
                        "FMU layout",
                        format!("malformed attribute in {}: {err}", path.display()),
                    )
                })?;
                let value = attribute
                    .normalized_value(XmlVersion::Explicit1_0)
                    .map_err(|err| {
                        validation_error(
                            "FMU layout",
                            format!("malformed attribute value in {}: {err}", path.display()),
                        )
                    })?;
                match (co_simulation, attribute.key.as_ref()) {
                    (false, "fmiVersion") => description.version = value.into_owned(),
                    (false, "guid") => description.guid = Some(value.into_owned()),
                    (false, "instantiationToken") => {
                        description.instantiation_token = Some(value.into_owned())
                    }
                    (true, "modelIdentifier") => {
                        description.model_identifier = Some(value.into_owned())
                    }
                    (true, "canGetAndSetFMUstate") if description.version == "2.0" => {
                        description.can_get_and_set_state = matches!(value.as_ref(), "true" | "1");
                    }
                    (true, "canGetAndSetFMUState") if description.version == "3.0" => {
                        description.can_get_and_set_state = matches!(value.as_ref(), "true" | "1");
                    }
                    (true, "canRunAsynchronuously") if description.version == "2.0" => {
                        description.can_run_asynchronously = matches!(value.as_ref(), "true" | "1");
                    }
                    _ => (),
                }
            }
        }
        found_co_simulation |= co_simulation;
        root_closed = !found_root && !open;
        found_root = true;
        depth += usize::from(open);
        if depth > MAX_XML_DEPTH {
            return Err(validation_error(
                "FMU layout",
                format!(
                    "{} exceeds maximum XML nesting depth of {MAX_XML_DEPTH}",
                    path.display()
                ),
            ));
        }
    }

    if !found_root {
        return Err(validation_error(
            "FMU layout",
            format!(
                "{} does not contain an fmiModelDescription root",
                path.display()
            ),
        ));
    }
    if depth != 0 {
        return Err(validation_error(
            "FMU layout",
            format!("malformed XML: unclosed XML tags in {}", path.display()),
        ));
    }
    match description.version.as_str() {
        "2.0" | "3.0" => Ok(description),
        _ => Err(validation_error(
            "FMU layout",
            format!("{} does not declare FMI version 2.0 or 3.0", path.display()),
        )),
    }
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

fn map_zip_error(path: &Path, err: zip::result::ZipError) -> FmiError {
    match err {
        zip::result::ZipError::CompressionMethodNotSupported(method) => {
            FmiError::UnsupportedArchiveCompression {
                path: path.to_path_buf(),
                method,
            }
        }
        zip::result::ZipError::UnsupportedArchive(msg) => {
            if let Some(rest) = msg.strip_prefix("compression method not supported: ") {
                if let Ok(method) = rest.trim().parse::<u16>() {
                    return FmiError::UnsupportedArchiveCompression {
                        path: path.to_path_buf(),
                        method,
                    };
                }
            }
            validation_error("FMU archive", err.to_string())
        }
        other => validation_error("FMU archive", other.to_string()),
    }
}

pub fn extract_zip_entries(path: &Path, destination: &Path) -> FmiResult<()> {
    if !path.is_file() {
        return Err(validation_error(
            "FMU archive",
            format!("archive path is not a file: {}", path.display()),
        ));
    }

    let mut directory = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        directory.mode(0o700);
    }
    directory
        .create(destination)
        .map_err(|error| io_error("create FMU extraction directory", destination.into(), error))?;

    let extraction_result = (|| -> FmiResult<()> {
        let file = fs::File::open(path)
            .map_err(|error| io_error("open FMU archive", path.to_path_buf(), error))?;
        let mut archive = zip::ZipArchive::new(file).map_err(|error| map_zip_error(path, error))?;

        if archive.is_empty() {
            return Err(validation_error(
                "FMU archive",
                "archive contains no entries",
            ));
        }

        if archive.has_duplicate_names() {
            return Err(validation_error(
                "FMU archive",
                "archive contains duplicate file names in central directory",
            ));
        }

        if archive.len() > MAX_ARCHIVE_FILES {
            return Err(validation_error(
                "FMU archive",
                format!(
                    "archive entry count {} exceeds maximum allowed {}",
                    archive.len(),
                    MAX_ARCHIVE_FILES
                ),
            ));
        }

        let mut seen_entries = HashSet::new();
        let mut total_uncompressed_bytes: u64 = 0;
        let mut extracted_file_count: usize = 0;

        for i in 0..archive.len() {
            let mut entry = archive
                .by_index(i)
                .map_err(|error| map_zip_error(path, error))?;

            let method_code = match entry.compression() {
                zip::CompressionMethod::Stored | zip::CompressionMethod::Deflated => None,
                #[allow(deprecated)]
                zip::CompressionMethod::Unsupported(code) => Some(code),
                #[allow(deprecated)]
                other => Some(other.to_u16()),
            };
            if let Some(method) = method_code {
                return Err(FmiError::UnsupportedArchiveCompression {
                    path: path.to_path_buf(),
                    method,
                });
            }

            let raw_name = entry
                .name()
                .map_err(|error| validation_error("FMU archive", error.to_string()))?
                .to_string();

            if entry.is_symlink() {
                return Err(FmiError::UnsafeArchiveEntry {
                    path: path.to_path_buf(),
                    entry: raw_name,
                });
            }
            let safe_relative = match entry.enclosed_name() {
                Some(name) => {
                    let verified = safe_archive_entry(path, &raw_name)?;
                    if verified != name {
                        return Err(FmiError::UnsafeArchiveEntry {
                            path: path.to_path_buf(),
                            entry: raw_name,
                        });
                    }
                    verified
                }
                None => {
                    return Err(FmiError::UnsafeArchiveEntry {
                        path: path.to_path_buf(),
                        entry: raw_name,
                    });
                }
            };

            if !seen_entries.insert(safe_relative.clone()) {
                return Err(validation_error(
                    "FMU archive",
                    format!("duplicate archive entry: {}", safe_relative.display()),
                ));
            }

            let target = destination.join(&safe_relative);

            if entry.is_dir() || raw_name.ends_with('/') {
                fs::create_dir_all(&target)
                    .map_err(|error| io_error("create FMU archive directory", target, error))?;
            } else {
                if total_uncompressed_bytes.saturating_add(entry.size()) > MAX_EXPANDED_BYTES {
                    return Err(validation_error(
                        "FMU archive",
                        format!(
                            "archive uncompressed size exceeds limit of {MAX_EXPANDED_BYTES} bytes"
                        ),
                    ));
                }

                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent).map_err(|error| {
                        io_error(
                            "create FMU archive parent directory",
                            parent.to_path_buf(),
                            error,
                        )
                    })?;
                }

                let mut out_file = fs::File::create(&target).map_err(|error| {
                    io_error("create FMU archive entry file", target.clone(), error)
                })?;

                let remaining_budget = MAX_EXPANDED_BYTES - total_uncompressed_bytes;
                let mut limited_reader = (&mut entry).take(remaining_budget.saturating_add(1));
                let copied =
                    std::io::copy(&mut limited_reader, &mut out_file).map_err(|error| {
                        io_error("extract FMU archive entry", target.clone(), error)
                    })?;

                if copied > remaining_budget {
                    return Err(validation_error(
                        "FMU archive",
                        format!(
                            "archive uncompressed stream exceeded limit of {MAX_EXPANDED_BYTES} bytes"
                        ),
                    ));
                }

                total_uncompressed_bytes += copied;
                extracted_file_count += 1;
            }
        }

        if extracted_file_count == 0 {
            return Err(validation_error(
                "FMU archive",
                "archive contains no file entries",
            ));
        }

        Ok(())
    })();

    if extraction_result.is_err() {
        let _ = fs::remove_dir_all(destination);
    }

    extraction_result
}

#[doc(hidden)]
pub fn extract_stored_zip_entries(path: &Path, destination: &Path) -> FmiResult<()> {
    extract_zip_entries(path, destination)
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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn fmi3_platform_uses_fmi_architecture_and_system_names() {
        assert_eq!(fmi3_platform("arm", "linux"), "aarch32-linux");
        assert_eq!(fmi3_platform("aarch64", "macos"), "aarch64-darwin");
        assert_eq!(fmi3_platform("x86_64", "linux"), "x86_64-linux");
    }

    #[cfg(unix)]
    #[test]
    fn rejects_existing_extraction_tree_and_destination_symlink() {
        use std::os::unix::fs::symlink;

        let root = std::env::temp_dir().join(format!(
            "kairo-fmu-symlink-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        let outside = root.join("outside");
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("victim"), b"original").unwrap();
        let archive = root.join("input.fmu");
        let name = b"resources/victim";
        let body = b"overwritten";
        let mut bytes = vec![0u8; 30];
        bytes[0..4].copy_from_slice(&0x0403_4b50u32.to_le_bytes());
        bytes[18..22].copy_from_slice(&(body.len() as u32).to_le_bytes());
        bytes[22..26].copy_from_slice(&(body.len() as u32).to_le_bytes());
        bytes[26..28].copy_from_slice(&(name.len() as u16).to_le_bytes());
        bytes.extend_from_slice(name);
        bytes.extend_from_slice(body);
        fs::write(&archive, bytes).unwrap();

        let destination = root.join("existing");
        fs::create_dir(&destination).unwrap();
        symlink(&outside, destination.join("resources")).unwrap();
        assert!(extract_stored_zip_entries(&archive, &destination).is_err());
        assert_eq!(fs::read(outside.join("victim")).unwrap(), b"original");

        let linked_destination = root.join("linked");
        symlink(&outside, &linked_destination).unwrap();
        assert!(extract_stored_zip_entries(&archive, &linked_destination).is_err());
        assert!(!outside.join("resources").exists());
        fs::remove_dir_all(root).unwrap();
    }

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
