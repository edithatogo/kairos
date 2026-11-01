// Compile the very same private allocator with std and Loom atomic types.
use std::{env, fs, path::PathBuf};

fn main() {
    let path = "../../../../crates/kairo-ecs-pdes/src/generation_bitset.rs";
    println!("cargo:rerun-if-changed={path}");
    let source = fs::read_to_string(path).expect("read PDES allocator");
    let start = source
        .find("\nfn allocate_instance_id(")
        .expect("allocator definition");
    let tail = &source[start + 1..];
    // The following test module is a top-level item. Do not infer the
    // allocator boundary from braces inside the function body.
    let (allocator, _) = tail
        .split_once("\n#[cfg(test)]\nmod tests {")
        .expect("review allocator extraction after source layout changes");
    let output = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("allocator.rs");
    fs::write(output, allocator).unwrap();
}
