use std::process::Command;
fn main() {
    println!("cargo:rerun-if-changed=../../Cargo.lock");
    for name in ["HEAD", "index"] {
        if let Ok(o) = Command::new("git")
            .args(["rev-parse", "--git-path", name])
            .output()
        {
            if o.status.success() {
                println!(
                    "cargo:rerun-if-changed={}",
                    String::from_utf8_lossy(&o.stdout).trim()
                );
            }
        }
    }
    if let Ok(o) = Command::new("git")
        .args(["symbolic-ref", "-q", "HEAD"])
        .output()
    {
        if o.status.success() {
            let name = String::from_utf8_lossy(&o.stdout);
            if let Ok(p) = Command::new("git")
                .args(["rev-parse", "--git-path", name.trim()])
                .output()
            {
                if p.status.success() {
                    println!(
                        "cargo:rerun-if-changed={}",
                        String::from_utf8_lossy(&p.stdout).trim()
                    );
                }
            }
        }
    }
    let dirty = Command::new("git")
        .args(["status", "--porcelain", "--untracked-files=no"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| if o.stdout.is_empty() { "false" } else { "true" })
        .unwrap_or("unavailable");
    println!("cargo:rustc-env=KAIROS_C5_SOURCE_DIRTY={dirty}");
    let sha = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        .unwrap_or_else(|| "unavailable".into());
    println!("cargo:rustc-env=KAIROS_C5_SOURCE_COMMIT={sha}");
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let version = Command::new(rustc)
        .arg("--version")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        .unwrap_or_else(|| "unavailable".into());
    println!("cargo:rustc-env=KAIROS_C5_TOOLCHAIN={version}");
}
