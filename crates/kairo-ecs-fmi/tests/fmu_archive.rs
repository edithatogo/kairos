use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use kairo_ecs_fmi::import::fmu_loader::{current_fmi_platform, FmuArchive};

fn temp_path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "kairo-fmi-{label}-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos()
    ))
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

fn stored_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for (name, body) in entries {
        let name = name.as_bytes();
        bytes.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        bytes.extend_from_slice(&20u16.to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&(body.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&(body.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&(name.len() as u16).to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes.extend_from_slice(name);
        bytes.extend_from_slice(body);
    }
    bytes
}

#[test]
fn fmu_archive_extracts_stored_zip_and_validates_layout() {
    let platform = current_fmi_platform();
    let archive_path = temp_path("stored.fmu");
    let destination = temp_path("stored-out");
    let binary_name = format!("binaries/{platform}/model.{}", shared_library_extension());
    let bytes = stored_zip(&[
        (
            "modelDescription.xml",
            br#"<fmiModelDescription fmiVersion="2.0" />"#,
        ),
        (&binary_name, b"fixture-binary"),
    ]);
    fs::write(&archive_path, bytes).expect("write archive");

    let layout = FmuArchive::new(&archive_path)
        .extract_to(&destination)
        .expect("extract archive");

    assert_eq!(layout.root(), destination.as_path());
    assert!(layout.model_description().is_file());
    assert!(layout
        .binary()
        .ends_with(format!("model.{}", shared_library_extension())));

    fs::remove_file(archive_path).expect("remove archive");
    fs::remove_dir_all(destination).expect("remove destination");
}

#[test]
fn fmu_archive_rejects_path_traversal_entries() {
    let archive_path = temp_path("traversal.fmu");
    let destination = temp_path("traversal-out");
    fs::write(
        &archive_path,
        stored_zip(&[("../escape.txt", b"must not be written")]),
    )
    .expect("write archive");

    let error = FmuArchive::new(&archive_path)
        .extract_to(&destination)
        .expect_err("reject traversal");

    assert!(error.to_string().contains("unsafe archive entry"));
    assert!(!Path::new(&destination)
        .join("..")
        .join("escape.txt")
        .exists());

    fs::remove_file(archive_path).expect("remove archive");
    let _ = fs::remove_dir_all(destination);
}
