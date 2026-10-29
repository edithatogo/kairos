fn main() {
    println!("cargo:rerun-if-changed=src/native.c");
    let target = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
    if !matches!(target.as_str(), "linux" | "macos" | "windows") {
        return;
    }
    let mut build = cc::Build::new();
    build
        .file("src/native.c")
        .std("c11")
        .warnings(true)
        .warnings_into_errors(true);
    if target == "macos" {
        build.define("_DARWIN_C_SOURCE", None);
    }
    if target == "windows" {
        build.define("_WIN32_WINNT", Some("0x0A00"));
    }
    build.compile("kairo_fs_guard");
    if target == "windows" {
        println!("cargo:rustc-link-lib=advapi32");
    }
}
