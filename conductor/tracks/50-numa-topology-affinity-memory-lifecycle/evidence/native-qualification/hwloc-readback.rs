use kairo_ecs_core::HwlocTopology;
use std::path::Path;

fn ids(cpus: &[u32]) -> String {
    cpus.iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn main() -> std::io::Result<()> {
    let program = std::env::args().nth(1).expect("absolute lstopo path");
    let topology = HwlocTopology::discover(Path::new(&program))?;
    println!("allowed\t{}", ids(&topology.allowed_cpus));
    for node in &topology.nodes {
        println!(
            "node\t{}\t{}\t{}",
            node.id,
            node.memory_bytes.unwrap(),
            ids(&node.cores)
        );
    }
    for cpus in &topology.core_cpus {
        println!("core\t{}", ids(cpus));
    }
    for cache in &topology.caches {
        println!(
            "cache\t{}\t{:?}\t{}\t{}\t{}",
            cache.level,
            cache.kind,
            cache.size_bytes,
            cache.line_bytes,
            ids(&cache.cpus)
        );
    }
    Ok(())
}
