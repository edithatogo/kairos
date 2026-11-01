#![cfg(all(feature = "numa", target_os = "linux"))]

use kairo_ecs_core::{AffinityBindingPlan, NumaError, NumaSupport, NumaTopology};
use std::fs;

fn allowed() -> String {
    fs::read_to_string("/proc/thread-self/status")
        .unwrap()
        .lines()
        .find_map(|line| line.strip_prefix("Cpus_allowed_list:"))
        .unwrap()
        .trim()
        .to_owned()
}

#[test]
fn kernel_topology_agrees_with_node_memory() {
    // Host discovery requires sysfs and an unoverridden kernel topology.
    if !std::path::Path::new("/sys/devices/system/node/online").exists()
        || std::env::var_os("KAIRO_NUMA_NODES").is_some()
    {
        return;
    }
    let topology = NumaTopology::discover();
    assert_eq!(topology.support, NumaSupport::Supported);
    assert_eq!(topology.source, "linux-sysfs");
    for node in &topology.nodes {
        let meminfo =
            fs::read_to_string(format!("/sys/devices/system/node/node{}/meminfo", node.id))
                .unwrap();
        let row = meminfo
            .lines()
            .find(|line| line.contains(" MemTotal:"))
            .unwrap();
        let fields: Vec<_> = row.split_whitespace().collect();
        assert_eq!(fields[4], "kB");
        assert_eq!(
            node.memory_bytes,
            Some(fields[3].parse::<u64>().unwrap() * 1024)
        );
        println!(
            "node {} logical CPUs {:?}, memory {:?}",
            node.id, node.cores, node.memory_bytes
        );
    }
}

#[test]
fn current_thread_binding_is_reversible_and_isolated() {
    // Minimal Linux images can omit util-linux or procfs.
    if !std::path::Path::new("/usr/bin/taskset").is_file()
        || !std::path::Path::new("/proc/thread-self/status").is_file()
    {
        return;
    }
    let original = allowed();
    let cpu = original.split([',', '-']).next().unwrap().parse().unwrap();
    assert_eq!(
        AffinityBindingPlan::new(0, cpu, false),
        Err(NumaError::InvalidAffinity)
    );
    let plan = AffinityBindingPlan::new(0, cpu, true).unwrap();
    let invalid = AffinityBindingPlan::new(0, u32::MAX, true).unwrap();
    assert!(invalid.apply().is_err());
    assert_eq!(allowed(), original);
    let caller_tid: u64 = fs::read_link("/proc/thread-self")
        .unwrap()
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .parse()
        .unwrap();
    std::thread::scope(|scope| {
        scope
            .spawn(|| {
                let before = allowed();
                let foreign = AffinityBindingPlan::new(caller_tid, cpu, true).unwrap();
                assert_eq!(foreign.apply(), Err(NumaError::InvalidAffinity));
                assert_eq!(allowed(), before);
            })
            .join()
            .unwrap();
    });
    plan.apply().unwrap();
    assert_eq!(allowed(), cpu.to_string());
    std::thread::scope(|scope| {
        scope
            .spawn(|| {
                let before = allowed();
                assert_eq!(plan.revert(), Err(NumaError::InvalidAffinity));
                assert_eq!(allowed(), before);
            })
            .join()
            .unwrap();
    });
    assert_eq!(allowed(), cpu.to_string());
    assert!(plan.apply().is_err());
    assert!(invalid.revert().is_err());
    plan.reversal().apply().unwrap();
    assert_eq!(allowed(), original);
    assert!(plan.revert().is_err());
    println!("original {original}, bound {cpu}, restored {}", allowed());
}
