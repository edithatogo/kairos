//! Narrow native permission boundary for local output directories.
//!
//! Only the current OS identity and administrators are trusted. This does not
//! isolate processes sharing an identity, hostile filesystem servers, or later
//! permission changes by a trusted identity. Checks never modify permissions.
#![deny(unsafe_op_in_unsafe_fn)]

use std::{fs::File, io};

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
extern "C" {
    fn kairo_fs_guard(handle: usize, private_directory: i32) -> i32;
}

#[cfg(all(test, windows))]
mod windows_tests {
    use super::*;
    use std::os::windows::fs::OpenOptionsExt;

    #[test]
    fn user_temporary_directory_is_protected() {
        let root = tempfile::tempdir().unwrap();
        let file = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(0x02000000) // FILE_FLAG_BACKUP_SEMANTICS for directories.
            .open(root.path())
            .unwrap();
        verify_directory(&file, false).unwrap();
        verify_directory(&file, true).unwrap();
    }
}

/// Require a directory owned by the current identity, protected from writes by
/// other users. With `private` true, other users must also lack read/traverse
/// access. ACLs and the underlying filesystem must be verifiable.
pub fn verify_directory(directory: &File, private: bool) -> io::Result<()> {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    let handle = {
        use std::os::fd::AsRawFd;
        directory.as_raw_fd() as usize
    };
    #[cfg(windows)]
    let handle = {
        use std::os::windows::io::AsRawHandle;
        directory.as_raw_handle() as usize
    };
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        let _ = (directory, private);
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "directory permission verification unavailable",
        ));
    }
    #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
    // SAFETY: the borrowed File keeps the native descriptor/handle alive for
    // the synchronous read-only call. No pointer or ownership is transferred.
    let status = unsafe { kairo_fs_guard(handle, i32::from(private)) };
    #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
    match status {
        0 => Ok(()),
        1 => Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "output directory is not owned by the current OS identity",
        )),
        2 => Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "output directory permissions allow access by untrusted users",
        )),
        3 => Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "output directory ACL is not accepted by the protected-root policy",
        )),
        5 => Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "output filesystem permission model is not qualified",
        )),
        6 => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "output root is not a local directory",
        )),
        _ => Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "native directory permissions could not be verified",
        )),
    }
}

#[cfg(all(test, any(target_os = "linux", target_os = "macos")))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn private_directory_and_mode_rejection() {
        let root = tempfile::tempdir().unwrap();
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let file = File::open(root.path()).unwrap();
        verify_directory(&file, true).unwrap();
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o777)).unwrap();
        assert_eq!(
            verify_directory(&file, false).unwrap_err().kind(),
            io::ErrorKind::PermissionDenied
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn extended_acl_cannot_hide_behind_private_mode() {
        let root = tempfile::tempdir().unwrap();
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let file = File::open(root.path()).unwrap();
        verify_directory(&file, true).unwrap();
        let status = std::process::Command::new("/bin/chmod")
            .args(["+a", "everyone allow add_file,delete_child"])
            .arg(root.path())
            .status()
            .unwrap();
        assert!(status.success());
        assert_eq!(
            verify_directory(&file, false).unwrap_err().kind(),
            io::ErrorKind::PermissionDenied
        );
    }
}
