use super::{AffinityAction, AffinityBindingPlan, NumaError, NumaNode, NumaSupport, NumaTopology};
use std::{cell::RefCell, fs, path::Path, process::Command};

fn cpu_list(list: &str) -> Option<Vec<u32>> {
    let mut cpus = Vec::new();
    if list.trim().is_empty() {
        return Some(cpus);
    }
    for part in list.trim().split(',') {
        let (first, last) = part.split_once('-').unwrap_or((part, part));
        let first: u32 = first.parse().ok()?;
        let last: u32 = last.parse().ok()?;
        if first > last || last >= 1_048_576 || cpus.last().is_some_and(|&n| n >= first) {
            return None;
        }
        cpus.extend(first..=last);
    }
    Some(cpus)
}

pub(super) fn discover(root: &Path) -> Option<NumaTopology> {
    let mut nodes = Vec::new();
    for entry in fs::read_dir(root).ok()? {
        let entry = entry.ok()?;
        let name = entry.file_name();
        let Some(id) = name
            .to_str()?
            .strip_prefix("node")
            .and_then(|n| n.parse().ok())
        else {
            continue;
        };
        let path = entry.path();
        let cores = cpu_list(&fs::read_to_string(path.join("cpulist")).ok()?)?;
        let memory_bytes = fs::read_to_string(path.join("meminfo"))
            .ok()
            .and_then(|info| {
                let prefix = format!("Node {id} MemTotal:");
                let kb = info
                    .lines()
                    .find_map(|line| line.strip_prefix(&prefix))?
                    .trim()
                    .strip_suffix(" kB")?
                    .trim()
                    .parse::<u64>()
                    .ok()?;
                kb.checked_mul(1024)
            });
        nodes.push(NumaNode {
            id,
            cores,
            memory_bytes,
        });
    }
    nodes.sort_by_key(|node| node.id);
    (!nodes.is_empty()).then(|| NumaTopology {
        support: NumaSupport::Supported,
        nodes,
        source: "linux-sysfs".into(),
    })
}

// Scope the saved mask to the calling OS thread. A recycled TID cannot inherit it.
thread_local! {
    static PREVIOUS: RefCell<Option<(u32, String)>> = const { RefCell::new(None) };
}

fn allowed() -> Result<String, NumaError> {
    fs::read_to_string("/proc/thread-self/status")
        .map_err(|_| NumaError::UnsupportedHost)?
        .lines()
        .find_map(|line| line.strip_prefix("Cpus_allowed_list:"))
        .map(|mask| mask.trim().to_owned())
        .ok_or(NumaError::UnsupportedHost)
}

fn set_mask(tid: u64, mask: &str) -> Result<(), NumaError> {
    let status = Command::new("/usr/bin/taskset")
        .args(["-pc", mask, &tid.to_string()])
        .output()
        .map_err(|_| NumaError::UnsupportedHost)?
        .status;
    if status.success() && allowed()? == mask {
        Ok(())
    } else {
        Err(NumaError::InvalidAffinity)
    }
}

pub(super) fn affinity(
    plan: &AffinityBindingPlan,
    action: AffinityAction,
) -> Result<(), NumaError> {
    let tid: u64 = fs::read_link("/proc/thread-self")
        .map_err(|_| NumaError::UnsupportedHost)?
        .file_name()
        .and_then(|s| s.to_str())
        .and_then(|s| s.parse().ok())
        .ok_or(NumaError::UnsupportedHost)?;
    if plan.thread_id != 0 && plan.thread_id != tid {
        return Err(NumaError::InvalidAffinity);
    }
    PREVIOUS.with_borrow_mut(|saved| match action {
        AffinityAction::Bind => {
            let original = allowed()?;
            if saved.is_some()
                || !cpu_list(&original)
                    .ok_or(NumaError::InvalidAffinity)?
                    .contains(&plan.core_id)
            {
                return Err(NumaError::InvalidAffinity);
            }
            *saved = Some((plan.core_id, original.clone()));
            if let Err(err) = set_mask(tid, &plan.core_id.to_string()) {
                if allowed().as_deref() == Ok(original.as_str()) || set_mask(tid, &original).is_ok()
                {
                    *saved = None;
                }
                return Err(err);
            }
            Ok(())
        }
        AffinityAction::RestorePrevious => {
            let (_, mask) = saved
                .as_ref()
                .filter(|(core, _)| *core == plan.core_id)
                .ok_or(NumaError::InvalidAffinity)?;
            set_mask(tid, mask)?;
            *saved = None;
            Ok(())
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovery_preserves_sparse_and_memory_only_nodes() {
        let root = std::env::temp_dir().join(format!(
            "kairos-numa-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        for (id, cpus) in [(7, ""), (2, "1-3,5")] {
            let node = root.join(format!("node{id}"));
            fs::create_dir(&node).unwrap();
            fs::write(node.join("cpulist"), cpus).unwrap();
            fs::write(
                node.join("meminfo"),
                format!("Node {id} MemTotal:  128 kB\n"),
            )
            .unwrap();
        }
        let topology = discover(&root).unwrap();
        assert_eq!(
            topology.nodes,
            vec![
                NumaNode {
                    id: 2,
                    cores: vec![1, 2, 3, 5],
                    memory_bytes: Some(131072)
                },
                NumaNode {
                    id: 7,
                    cores: vec![],
                    memory_bytes: Some(131072)
                },
            ]
        );
        fs::write(root.join("node2/cpulist"), "3-1").unwrap();
        assert!(discover(&root).is_none());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn sparse_cpu_lists_preserve_ids_and_reject_malformed_ranges() {
        assert_eq!(cpu_list("1,4-6,10\n"), Some(vec![1, 4, 5, 6, 10]));
        assert_eq!(cpu_list("\n"), Some(vec![])); // CPU-less memory node.
        for malformed in ["1-0", "0-2,2-5", "0-4294967295", "3,", "a"] {
            assert_eq!(cpu_list(malformed), None);
        }
    }
}
