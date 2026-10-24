#![cfg(feature = "fmi2")]

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use kairo_ecs_fmi::export::model_description::{ModelDescription, ScalarVariable};
use kairo_ecs_fmi::export::packager::{package_fmu_archive, write_unpacked_fmu};
use kairo_ecs_fmi::import::fmu_loader::{
    current_fmi_platform, FmuArchive, MAX_ARCHIVE_FILES, MAX_EXPANDED_BYTES,
};
use kairo_ecs_fmi::FmiError;

fn temp_path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "kairo-fmi-oracle-{label}-{}-{}",
        std::process::id(),
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

fn create_test_fmu(
    method: zip::CompressionMethod,
    large_file: bool,
    extra_entries: &[(&str, &[u8])],
) -> Vec<u8> {
    let platform = current_fmi_platform();
    let binary_name = format!("binaries/{platform}/model.{}", shared_library_extension());
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(method)
            .large_file(large_file);

        writer
            .start_file("modelDescription.xml", options)
            .expect("start modelDescription.xml");
        writer
            .write_all(br#"<fmiModelDescription fmiVersion="2.0" />"#)
            .expect("write modelDescription.xml");

        writer
            .start_file(&binary_name, options)
            .expect("start binary");
        writer
            .write_all(b"dummy-binary-bytes")
            .expect("write binary");

        for (name, data) in extra_entries {
            writer.start_file(*name, options).expect("start extra");
            writer.write_all(data).expect("write extra");
        }
        writer.finish().expect("finish");
    }
    cursor.into_inner()
}

#[test]
fn oracle_compressed_deflate_extraction_and_layout() {
    let archive_path = temp_path("deflate.fmu");
    let destination = temp_path("deflate-out");
    let bytes = create_test_fmu(
        zip::CompressionMethod::Deflated,
        false,
        &[("resources/config.txt", b"deflated-config-content")],
    );
    fs::write(&archive_path, bytes).expect("write deflate archive");

    let layout = FmuArchive::new(&archive_path)
        .extract_to(&destination)
        .expect("extract deflate archive");

    assert_eq!(layout.root(), destination.as_path());
    assert!(layout.model_description().is_file());
    assert!(layout.binary().is_file());
    assert_eq!(
        fs::read(destination.join("resources/config.txt")).expect("read config"),
        b"deflated-config-content"
    );

    let _ = fs::remove_file(archive_path);
    let _ = fs::remove_dir_all(destination);
}

#[test]
fn oracle_stored_archive_backward_compatibility() {
    let archive_path = temp_path("stored.fmu");
    let destination = temp_path("stored-out");
    let bytes = create_test_fmu(
        zip::CompressionMethod::Stored,
        false,
        &[("resources/notes.txt", b"stored-uncompressed-notes")],
    );
    fs::write(&archive_path, bytes).expect("write stored archive");

    let layout = FmuArchive::new(&archive_path)
        .extract_to(&destination)
        .expect("extract stored archive");

    assert_eq!(layout.root(), destination.as_path());
    assert!(layout.model_description().is_file());
    assert_eq!(
        fs::read(destination.join("resources/notes.txt")).expect("read notes"),
        b"stored-uncompressed-notes"
    );

    let _ = fs::remove_file(archive_path);
    let _ = fs::remove_dir_all(destination);
}

#[test]
fn oracle_zip64_archive_extraction() {
    let archive_path = temp_path("zip64.fmu");
    let destination = temp_path("zip64-out");
    let bytes = create_test_fmu(zip::CompressionMethod::Deflated, true, &[]);
    fs::write(&archive_path, bytes).expect("write zip64 archive");

    let layout = FmuArchive::new(&archive_path)
        .extract_to(&destination)
        .expect("extract zip64 archive");

    assert_eq!(layout.root(), destination.as_path());
    assert!(layout.model_description().is_file());

    let _ = fs::remove_file(archive_path);
    let _ = fs::remove_dir_all(destination);
}

#[test]
fn oracle_path_traversal_rejection_variations() {
    let variations = [
        "../escaped_parent.txt",
        "nested/../../escaped_two_levels.txt",
        "/absolute_path.txt",
    ];

    for entry_name in variations {
        let archive_path = temp_path("traversal.fmu");
        let destination = temp_path("traversal-out");
        let mut cursor = std::io::Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut cursor);
            let options = zip::write::SimpleFileOptions::default();
            writer.start_file(entry_name, options).expect("start");
            writer.write_all(b"malicious").expect("write");
            writer.finish().expect("finish");
        }
        fs::write(&archive_path, cursor.into_inner()).expect("write traversal fmu");

        let result = FmuArchive::new(&archive_path).extract_to(&destination);
        match result {
            Err(FmiError::UnsafeArchiveEntry { entry, .. }) => {
                assert!(entry.contains("escape") || entry.contains("absolute"));
            }
            other => panic!("expected UnsafeArchiveEntry, got {other:?}"),
        }

        assert!(
            !destination.exists(),
            "destination must be cleaned up on failure"
        );
        let _ = fs::remove_file(archive_path);
    }
}

#[test]
fn oracle_symlink_rejection() {
    let archive_path = temp_path("symlink.fmu");
    let destination = temp_path("symlink-out");
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut cursor);
        let options = zip::write::SimpleFileOptions::default();
        writer
            .add_symlink("symlink_entry", "/etc/passwd", options)
            .expect("add symlink");
        writer.finish().expect("finish");
    }
    fs::write(&archive_path, cursor.into_inner()).expect("write symlink fmu");

    let result = FmuArchive::new(&archive_path).extract_to(&destination);
    match result {
        Err(FmiError::UnsafeArchiveEntry { entry, .. }) => {
            assert_eq!(entry, "symlink_entry");
        }
        other => panic!("expected UnsafeArchiveEntry for symlink, got {other:?}"),
    }

    assert!(
        !destination.exists(),
        "destination must be cleaned up on symlink rejection"
    );
    let _ = fs::remove_file(archive_path);
}

#[test]
fn oracle_duplicate_entries_rejection() {
    let archive_path = temp_path("duplicate.fmu");
    let destination = temp_path("duplicate-out");
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut cursor);
        let options = zip::write::SimpleFileOptions::default();
        writer
            .start_file("modelDescription.xml", options)
            .expect("1");
        writer.write_all(b"first").expect("w1");
        // "otherDescription.xml" has exactly 20 bytes, matching "modelDescription.xml"
        writer
            .start_file("otherDescription.xml", options)
            .expect("2");
        writer.write_all(b"second").expect("w2");
        writer.finish().expect("finish");
    }
    let mut bytes = cursor.into_inner();
    let target = b"otherDescription.xml";
    let replacement = b"modelDescription.xml";
    let mut replaced_count = 0;
    for i in 0..(bytes.len() - target.len()) {
        if &bytes[i..i + target.len()] == target {
            bytes[i..i + target.len()].copy_from_slice(replacement);
            replaced_count += 1;
        }
    }
    assert!(
        replaced_count >= 2,
        "must replace in local and central directory headers"
    );
    fs::write(&archive_path, bytes).expect("write duplicate fmu");

    let result = FmuArchive::new(&archive_path).extract_to(&destination);
    match result {
        Err(FmiError::Validation { artifact, message }) => {
            assert_eq!(artifact, "FMU archive");
            assert!(
                message.to_lowercase().contains("duplicate"),
                "expected duplicate message, got: {message}"
            );
        }
        other => panic!("expected Validation error for duplicate entries, got {other:?}"),
    }

    assert!(!destination.exists());
    let _ = fs::remove_file(archive_path);
}

#[test]
fn oracle_file_count_budget_enforcement() {
    let archive_path = temp_path("file-count.fmu");
    let destination = temp_path("file-count-out");

    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut cursor);
        let options = zip::write::SimpleFileOptions::default();
        for i in 0..=(MAX_ARCHIVE_FILES) {
            writer
                .start_file(format!("file_{i}.txt"), options)
                .expect("start");
            writer.write_all(b"x").expect("write");
        }
        writer.finish().expect("finish");
    }
    fs::write(&archive_path, cursor.into_inner()).expect("write count fmu");

    let result = FmuArchive::new(&archive_path).extract_to(&destination);
    match result {
        Err(FmiError::Validation { artifact, message }) => {
            assert_eq!(artifact, "FMU archive");
            assert!(
                message.contains("exceeds maximum allowed"),
                "expected file count error, got: {message}"
            );
        }
        other => panic!("expected Validation error for excessive file count, got {other:?}"),
    }

    assert!(!destination.exists());
    let _ = fs::remove_file(archive_path);
}

#[test]
fn oracle_expansion_limit_enforcement() {
    let archive_path = temp_path("expansion-limit.fmu");
    let destination = temp_path("expansion-limit-out");

    let platform = current_fmi_platform();
    let binary_name = format!("binaries/{platform}/model.{}", shared_library_extension());
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        writer.start_file("modelDescription.xml", options).unwrap();
        writer
            .write_all(br#"<fmiModelDescription fmiVersion="2.0" />"#)
            .unwrap();
        writer.start_file(&binary_name, options).unwrap();
        writer.write_all(b"bin").unwrap();

        writer.start_file("large.bin", options).unwrap();
        writer.write_all(b"test").unwrap();
        writer.finish().unwrap();
    }
    let mut bytes = cursor.into_inner();
    let excessive_size: u32 = (MAX_EXPANDED_BYTES as u32).saturating_add(1024);
    for i in 0..(bytes.len() - 9) {
        if &bytes[i..i + 9] == b"large.bin" {
            if i >= 13 && bytes[i - 13..i - 9] == 4u32.to_le_bytes() {
                bytes[i - 9..i - 5].copy_from_slice(&excessive_size.to_le_bytes());
            }
            if i >= 22 && bytes[i - 26..i - 22] == 4u32.to_le_bytes() {
                bytes[i - 22..i - 18].copy_from_slice(&excessive_size.to_le_bytes());
            }
        }
    }
    fs::write(&archive_path, bytes).expect("write bomb fmu");

    let result = FmuArchive::new(&archive_path).extract_to(&destination);
    match result {
        Err(FmiError::Validation { artifact, message }) => {
            assert_eq!(artifact, "FMU archive");
            assert!(
                message.contains("exceeds limit") || message.contains("exceeded limit"),
                "expected expansion limit error, got: {message}"
            );
        }
        other => panic!("expected Validation error for expansion limit, got {other:?}"),
    }

    assert!(
        !destination.exists(),
        "partial extraction must be cleaned up on expansion overflow"
    );
    let _ = fs::remove_file(archive_path);
}

#[test]
fn oracle_corrupted_crc_and_cleanup_on_failure() {
    let archive_path = temp_path("corrupt.fmu");
    let destination = temp_path("corrupt-out");
    let mut bytes = create_test_fmu(
        zip::CompressionMethod::Deflated,
        false,
        &[("resources/data.bin", &[1, 2, 3, 4, 5, 6, 7, 8])],
    );

    // Corrupt compressed payload bytes in the middle of the archive
    let len = bytes.len();
    bytes[len / 2] ^= 0xFF;
    bytes[(len / 2) + 1] ^= 0xAA;

    fs::write(&archive_path, bytes).expect("write corrupt fmu");

    let result = FmuArchive::new(&archive_path).extract_to(&destination);
    assert!(result.is_err(), "must fail on corrupted archive");
    assert!(
        !destination.exists(),
        "destination directory must be removed on failure"
    );

    let _ = fs::remove_file(archive_path);
}

#[test]
fn oracle_unsupported_compression_method() {
    let archive_path = temp_path("unsupported-compression.fmu");
    let destination = temp_path("unsupported-compression-out");

    // Build archive with stored method, then overwrite compression method field to 12 (BZIP2)
    let mut bytes = create_test_fmu(zip::CompressionMethod::Stored, false, &[]);
    // In local header (first entry): compression method is at offset 8..10
    bytes[8] = 12;
    bytes[9] = 0;
    // Find central directory header signature 0x02014b50 and patch method at offset 10..12
    for i in 0..(bytes.len() - 4) {
        if bytes[i..i + 4] == [0x50, 0x4b, 0x01, 0x02] {
            bytes[i + 10] = 12;
            bytes[i + 11] = 0;
        }
    }

    fs::write(&archive_path, bytes).expect("write unsupported fmu");

    let result = FmuArchive::new(&archive_path).extract_to(&destination);
    match result {
        Err(FmiError::UnsupportedArchiveCompression { method, .. }) => {
            assert_eq!(method, 12);
        }
        other => panic!("expected UnsupportedArchiveCompression, got {other:?}"),
    }

    assert!(!destination.exists());
    let _ = fs::remove_file(archive_path);
}

#[test]
fn oracle_fmu_packager_roundtrip() {
    let unpacked_root = temp_path("packager-unpacked");
    let package_path = temp_path("exported.fmu");
    let destination = temp_path("packager-destination");

    let model_description = ModelDescription::new("thermal_engine", "{fmu-guid-1234}")
        .with_variable(ScalarVariable::real_output("temperature", 1));

    let layout =
        write_unpacked_fmu(&unpacked_root, &model_description).expect("write unpacked fmu");

    let platform = current_fmi_platform();
    let binary_dir = layout.root.join("binaries").join(&platform);
    fs::create_dir_all(&binary_dir).expect("create binary dir");
    let binary_file = binary_dir.join(format!("model.{}", shared_library_extension()));
    let binary_payload = b"compiled-sim-model-bytecode-2026";
    fs::write(&binary_file, binary_payload).expect("write binary");

    let extra_resource = layout.resources_dir.join("thermal_map.dat");
    let resource_payload = b"\x00\x01\x02\x03\x04\x05\x06\x07";
    fs::write(&extra_resource, resource_payload).expect("write resource");

    package_fmu_archive(&unpacked_root, &package_path).expect("package fmu archive");
    assert!(package_path.is_file(), "package file exists");

    let extracted_layout = FmuArchive::new(&package_path)
        .extract_to(&destination)
        .expect("extract packaged fmu");

    assert_eq!(extracted_layout.root(), destination.as_path());
    assert!(extracted_layout.model_description().is_file());
    assert_eq!(
        fs::read(extracted_layout.binary()).expect("read extracted binary"),
        binary_payload
    );
    assert_eq!(
        fs::read(destination.join("resources/thermal_map.dat")).expect("read resource"),
        resource_payload
    );

    let _ = fs::remove_dir_all(unpacked_root);
    let _ = fs::remove_file(package_path);
    let _ = fs::remove_dir_all(destination);
}
