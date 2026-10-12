#[cfg(feature = "flow")]
fn main() {
    use kairo_ecs_calibration::experimental_c2_replay::{restore_demo, save_demo};
    use std::path::Path;

    let mut args = std::env::args_os().skip(1);
    let Some(command) = args.next() else {
        usage();
        std::process::exit(2);
    };
    let Some(path) = args.next() else {
        usage();
        std::process::exit(2);
    };
    if args.next().is_some() {
        usage();
        std::process::exit(2);
    }

    let path = Path::new(&path);
    let result = match command.to_str() {
        Some("save") => save_demo(path).map(|()| {
            println!(
                "saved sealed C2 checkpoint at paused tick 1: {}",
                path.display()
            );
        }),
        Some("restore") => restore_demo(path).map(|trace| println!("{trace}")),
        _ => {
            usage();
            std::process::exit(2);
        }
    };
    if let Err(error) = result {
        eprintln!("C2 replay demo failed: {error}");
        std::process::exit(1);
    }
}

#[cfg(not(feature = "flow"))]
fn main() {
    eprintln!("enable the Flow feature: cargo run -p kairo-ecs-calibration --features flow --example c2_replay_demo -- <save|restore> CHECKPOINT");
    std::process::exit(2);
}

#[cfg(feature = "flow")]
fn usage() {
    eprintln!("usage: c2_replay_demo <save|restore> CHECKPOINT");
}
