use std::path::PathBuf;

use kairo_ecs_fmi::import::fmu_loader::FmuLayout;

fn main() {
    let Some(root) = std::env::args_os().nth(1).map(PathBuf::from) else {
        eprintln!("usage: kairo-fmi-basic-import <unpacked-fmu-root>");
        std::process::exit(2);
    };

    match FmuLayout::from_unpacked_dir(&root) {
        Ok(layout) => {
            println!("modelDescription={}", layout.model_description().display());
            println!("platform={}", layout.platform());
            println!("binary={}", layout.binary().display());
        }
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
