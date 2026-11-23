use std::env;

fn main() {
    if env::var_os("CARGO_FEATURE_NATIVE").is_some()
        && matches!(
            env::var("CARGO_CFG_TARGET_OS").as_deref(),
            Ok("linux" | "macos")
        )
    {
        println!("cargo:rerun-if-env-changed=ADIOS2_LIB_DIR");
        if let Some(dir) = env::var_os("ADIOS2_LIB_DIR") {
            println!("cargo:rustc-link-search=native={}", dir.to_string_lossy());
        }
        println!("cargo:rustc-link-lib=dylib=adios2_c");
    }
}
