use std::borrow::Cow;
use std::io::{self, ErrorKind};
use std::path::Path;
use std::process::Command;

use quick_xml::events::{BytesStart, Event};
use quick_xml::name::QName;
use quick_xml::Reader;

use crate::NumaNode;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HwlocCacheKind {
    Unified,
    Data,
    Instruction,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HwlocCache {
    pub level: u32,
    pub kind: HwlocCacheKind,
    pub size_bytes: u64,
    pub line_bytes: u32,
    /// OS CPU IDs sharing this cache.
    pub cpus: Vec<u32>,
}

/// CPU and memory topology observed through the hwloc 2.x XML export.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HwlocTopology {
    /// Memory domains with their associated OS CPU IDs in `NumaNode::cores`.
    pub nodes: Vec<NumaNode>,
    /// OS CPU IDs grouped by physical core. SMT siblings share a group.
    pub core_cpus: Vec<Vec<u32>>,
    pub caches: Vec<HwlocCache>,
    /// CPU mask reported by hwloc for this discovery.
    /// The calling thread can have a narrower affinity mask.
    pub allowed_cpus: Vec<u32>,
}

impl HwlocTopology {
    /// Discover the current machine with a trusted absolute path to
    /// `lstopo-no-graphics` from hwloc 2.x.
    ///
    /// The child uses a clean environment and reports observed topology.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if the tool cannot run, `InvalidInput` for a
    /// relative tool path, or `InvalidData` for an incompatible topology.
    pub fn discover(lstopo: &Path) -> io::Result<Self> {
        if !lstopo.is_absolute() {
            return Err(io::Error::new(
                ErrorKind::InvalidInput,
                "hwloc discovery requires an absolute executable path",
            ));
        }
        let output = Command::new(lstopo)
            .args(["--of", "xml", "-"])
            .env_clear()
            .output()?;
        if !output.status.success() {
            return Err(io::Error::other(format!(
                "hwloc discovery failed with {}",
                output.status
            )));
        }
        parse(&output.stdout)
    }
}

fn invalid_topology() -> io::Error {
    io::Error::new(ErrorKind::InvalidData, "invalid hwloc 2.x topology XML")
}

fn optional_attribute<'a>(tag: &'a BytesStart<'_>, name: &str) -> io::Result<Option<Cow<'a, str>>> {
    tag.try_get_attribute(name)
        .map(|attribute| attribute.map(|attribute| attribute.value))
        .map_err(|_| invalid_topology())
}

fn attribute<'a>(tag: &'a BytesStart<'_>, name: &str) -> io::Result<Cow<'a, str>> {
    optional_attribute(tag, name)?.ok_or_else(invalid_topology)
}

fn number<T: std::str::FromStr>(tag: &BytesStart<'_>, name: &str) -> io::Result<T> {
    attribute(tag, name)?
        .parse()
        .map_err(|_| invalid_topology())
}

fn cpu_ids(bitmap: &str) -> io::Result<Vec<u32>> {
    let mut cpus = Vec::new();
    for (group, word) in bitmap.rsplit(',').enumerate() {
        if group >= 32_768 {
            return Err(invalid_topology());
        }
        // hwloc may leave a zero 32-bit word empty between commas.
        if word.is_empty() {
            continue;
        }
        let hex = word.strip_prefix("0x").ok_or_else(invalid_topology)?;
        let mut bits = u32::from_str_radix(hex, 16).map_err(|_| invalid_topology())?;
        while bits != 0 {
            let bit = bits.trailing_zeros();
            cpus.push(group as u32 * 32 + bit);
            bits &= bits - 1;
        }
    }
    Ok(cpus)
}

fn parse(xml: &[u8]) -> io::Result<HwlocTopology> {
    let mut reader = Reader::from_reader(xml);
    let mut nodes = Vec::new();
    let mut core_cpus = Vec::new();
    let mut caches = Vec::new();
    let mut allowed_cpus = None;
    let mut opened = false;
    let mut closed = false;
    loop {
        match reader.read_event().map_err(|_| invalid_topology())? {
            Event::Start(tag) if tag.name() == QName("topology") => {
                if opened || attribute(&tag, "version")? != "2.0" {
                    return Err(invalid_topology());
                }
                opened = true;
            }
            Event::End(tag) if tag.name() == QName("topology") => closed = true,
            Event::Start(tag) | Event::Empty(tag) if tag.name() == QName("object") => {
                if !opened || closed {
                    return Err(invalid_topology());
                }
                let object = attribute(&tag, "type")?;
                match object.as_ref() {
                    "Machine" if allowed_cpus.is_none() => {
                        allowed_cpus = Some(cpu_ids(&attribute(&tag, "allowed_cpuset")?)?);
                    }
                    "Machine" => return Err(invalid_topology()),
                    "NUMANode" => nodes.push(NumaNode {
                        id: number(&tag, "os_index")?,
                        cores: cpu_ids(&attribute(&tag, "cpuset")?)?,
                        memory_bytes: optional_attribute(&tag, "local_memory")?
                            .map(|memory| memory.parse())
                            .transpose()
                            .map_err(|_| invalid_topology())?,
                    }),
                    "Core" => core_cpus.push(cpu_ids(&attribute(&tag, "cpuset")?)?),
                    name if name.starts_with('L') && name.ends_with("Cache") => {
                        let kind = match number::<u8>(&tag, "cache_type")? {
                            0 => HwlocCacheKind::Unified,
                            1 => HwlocCacheKind::Data,
                            2 => HwlocCacheKind::Instruction,
                            _ => return Err(invalid_topology()),
                        };
                        caches.push(HwlocCache {
                            level: number(&tag, "depth")?,
                            kind,
                            size_bytes: number(&tag, "cache_size")?,
                            line_bytes: number(&tag, "cache_linesize")?,
                            cpus: cpu_ids(&attribute(&tag, "cpuset")?)?,
                        });
                    }
                    _ => {}
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    nodes.sort_unstable_by_key(|node| node.id);
    if !closed
        || nodes.is_empty()
        || nodes.windows(2).any(|pair| pair[0].id == pair[1].id)
        || core_cpus.is_empty()
        || core_cpus.iter().any(Vec::is_empty)
    {
        return Err(invalid_topology());
    }
    Ok(HwlocTopology {
        nodes,
        core_cpus,
        caches,
        allowed_cpus: allowed_cpus.ok_or_else(invalid_topology)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const XML: &str = r#"<topology version="2.0">
          <object type="Machine" allowed_cpuset="0x00000002,,0x00000009">
            <object type="NUMANode" os_index="0" local_memory="64" cpuset="0x00000002,,0x00000009"/>
            <object type="NUMANode" os_index="2" local_memory="256" cpuset="0x0"/>
            <object type="NUMANode" os_index="3" cpuset="0x00000002,,0x0"/>
            <object type="L3Cache" depth="3" cache_size="4096" cache_linesize="64"
                    cache_type="0" cpuset="0x00000009">
              <object type="Core" cpuset="0x00000009"/>
            </object>
            <object type="Core" cpuset="0x00000002,,0x0"/>
          </object>
        </topology>"#;

    #[test]
    fn reads_shared_caches_and_memory_only_nodes_and_rejects_bad_topology() {
        let xml = XML.as_bytes();
        let topology = parse(xml).unwrap();
        assert_eq!(topology.allowed_cpus, [0, 3, 65]);
        assert_eq!(topology.core_cpus, [vec![0, 3], vec![65]]);
        assert_eq!(topology.nodes[1].id, 2);
        assert_eq!(topology.nodes[1].memory_bytes, Some(256));
        assert!(topology.nodes[1].cores.is_empty());
        assert_eq!(topology.nodes[2].cores, [65]);
        assert_eq!(topology.nodes[2].memory_bytes, None);
        assert_eq!(topology.caches[0].kind, HwlocCacheKind::Unified);
        assert_eq!(topology.caches[0].cpus, [0, 3]);
        assert_eq!(topology.caches[0].size_bytes, 4096);
        assert_eq!(
            parse(&xml[..xml.len() - 4]).unwrap_err().kind(),
            ErrorKind::InvalidData
        );
        let invalid_memory = XML.replace("local_memory=\"256\"", "local_memory=\"unknown\"");
        assert_eq!(
            parse(invalid_memory.as_bytes()).unwrap_err().kind(),
            ErrorKind::InvalidData
        );
        assert_eq!(
            HwlocTopology::discover(Path::new("relative/lstopo"))
                .unwrap_err()
                .kind(),
            ErrorKind::InvalidInput
        );
    }

    #[cfg(unix)]
    #[test]
    fn discovery_uses_the_explicit_program_and_checks_its_status() {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;

        let temporary = tempfile::tempdir().unwrap();
        let program = temporary.path().join("lstopo-fixture");
        assert_eq!(
            HwlocTopology::discover(&program).unwrap_err().kind(),
            ErrorKind::NotFound
        );
        let body =
            format!("#!/bin/sh\n[ \"$*\" = '--of xml -' ] || exit 17\nprintf '%s' '{XML}'\n");
        fs::write(&program, body).unwrap();
        fs::set_permissions(&program, fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(
            HwlocTopology::discover(&program).unwrap(),
            parse(XML.as_bytes()).unwrap()
        );
        fs::write(&program, "#!/bin/sh\nexit 7\n").unwrap();
        assert_eq!(
            HwlocTopology::discover(&program).unwrap_err().kind(),
            ErrorKind::Other
        );
    }
}
