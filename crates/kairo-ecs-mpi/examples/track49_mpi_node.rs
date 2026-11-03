// SPDX-License-Identifier: Apache-2.0
//! Owned synthetic MPI allocation. Every command is collective; no live rank recovery.
#[cfg(all(unix, feature = "external-fault-injection"))]
#[path = "../../../docs/distributed/track49-fixtures/model.rs"]
mod model;
#[cfg(all(unix, feature = "external-fault-injection"))]
mod worker {
    use super::model;
    use kairo_ecs_mpi::native::{NativeMpiExchange, NativeMpiRecord};
    use kairo_ecs_pdes::external::protocol::{digest, verify};
    use kairo_ecs_pdes::external::ExternalConfiguration;
    use mpi::traits::*;
    use serde_json::{json, Value};
    use std::collections::BTreeMap;
    use std::io::{BufRead, Write};
    use std::path::Path;

    fn gather(
        world: &mpi::topology::SimpleCommunicator,
        value: &Value,
    ) -> Result<Vec<Value>, String> {
        let raw = serde_json::to_vec(value).map_err(|e| e.to_string())?;
        let length = u32::try_from(raw.len()).map_err(|e| e.to_string())?;
        let mut lengths = vec![0u32; world.size() as usize];
        world.all_gather_into(&length, &mut lengths[..]);
        if lengths.iter().any(|n| *n > 8 * 1024 * 1024) {
            return Err("owned result bound".into());
        }
        let width = lengths.iter().copied().max().unwrap_or(0).max(1) as usize;
        let mut sent = vec![0u8; width];
        sent[..raw.len()].copy_from_slice(&raw);
        let mut received = vec![0u8; width * lengths.len()];
        world.all_gather_into(&sent[..], &mut received[..]);
        lengths
            .into_iter()
            .enumerate()
            .map(|(rank, n)| {
                serde_json::from_slice(&received[rank * width..rank * width + n as usize])
                    .map_err(|e| e.to_string())
            })
            .collect()
    }
    pub fn run() -> Result<(), Box<dyn std::error::Error>> {
        let arg = std::env::args().nth(1).ok_or("settings path required")?;
        if arg == "--describe" {
            println!("{}", model::describe(4));
            return Ok(());
        }
        if arg == "--serial" {
            println!("{}", model::serial(4)?);
            return Ok(());
        }
        let raw = std::fs::read(&arg)?;
        if raw.len() > 65536 {
            return Err("settings bound".into());
        }
        let settings: Value = serde_json::from_slice(&raw)?;
        let config: ExternalConfiguration = model::field(&settings, "configuration")?;
        let base: String = model::field(&settings, "stores")?;
        let universe = mpi::initialize().ok_or("owned MPI initialization")?;
        let world = universe.world();
        let exchange = NativeMpiExchange::new(&universe)?;
        let rank = exchange.rank();
        let size = exchange.size();
        if ![2, 4].contains(&size)
            || config.members.len() != size as usize
            || config
                .members
                .iter()
                .enumerate()
                .any(|(i, m)| m.participant != i as u32 || m.lps != vec![i as u32])
        {
            return Err("fixed rank configuration".into());
        }
        let keys = config
            .members
            .iter()
            .map(|m| (m.participant, m.public_key.clone()))
            .collect::<BTreeMap<_, _>>();
        let context = digest(&serde_json::to_vec(&config)?);
        let mut runtime = model::open(
            &Path::new(&base).join(format!("node{rank}")),
            config.clone(),
            rank,
        )?;
        let ready = gather(
            &world,
            &json!({"rank":rank,"pid":std::process::id(),"generation":runtime.generation(),"active":runtime.is_active(),"processor":mpi::environment::processor_name()?}),
        )?;
        if rank == 0 {
            println!("{}", json!({"ready":true,"size":size,"ranks":ready}));
            std::io::stdout().flush()?;
        }
        let mut lines = std::io::stdin().lock().lines();
        loop {
            let mut raw = if rank == 0 {
                lines
                    .next()
                    .transpose()?
                    .unwrap_or_else(|| "{\"op\":\"stop\"}".into())
                    .into_bytes()
            } else {
                vec![]
            };
            let mut length = u32::try_from(raw.len())?;
            world.process_at_rank(0).broadcast_into(&mut length);
            if length > 1024 * 1024 {
                return Err("owned command bound".into());
            }
            if rank != 0 {
                raw.resize(length as usize, 0);
            }
            world.process_at_rank(0).broadcast_into(&mut raw[..]);
            let command: Value = serde_json::from_slice(&raw)?;
            let op = command.get("op").and_then(Value::as_str).ok_or("op")?;
            if op == "stop" {
                break;
            }
            let target: u32 = model::field(&command, "rank")?;
            let mut result = Ok(json!({}));
            if op == "send" {
                let destination: u32 = model::field(&command, "destination")?;
                let packet: Vec<u8> = model::field(&command, "packet")?;
                let accept = command
                    .get("accept")
                    .and_then(Value::as_bool)
                    .unwrap_or(true);
                let records = if rank == target {
                    vec![NativeMpiRecord {
                        destination,
                        bytes: packet,
                    }]
                } else {
                    vec![]
                };
                let validate =
                    |source: u32, _destination: u32, bytes: &[u8]| -> std::io::Result<()> {
                        let record =
                            verify(bytes, context, &keys).map_err(std::io::Error::other)?;
                        if record.issuer != source {
                            return Err(std::io::Error::other(
                                "MPI source rank does not match signed issuer",
                            ));
                        }
                        Ok(())
                    };
                match exchange.exchange_records_validated(&records, validate) {
                    Err(e) => result = Err(e.to_string()),
                    Ok(incoming) => {
                        let mut replies = Vec::new();
                        for entry in incoming {
                            match runtime.receive_from(
                                config.members[entry.source_rank as usize].certificate_hash,
                                &entry.bytes,
                            ) {
                                Ok(packets) => replies.extend(packets.into_iter().map(|bytes| {
                                    NativeMpiRecord {
                                        destination: entry.source_rank,
                                        bytes,
                                    }
                                })),
                                Err(e) => result = Err(e.to_string()),
                            }
                        }
                        match exchange.exchange_records_validated(&replies, validate) {
                            Err(e) => result = Err(e.to_string()),
                            Ok(replies) => {
                                let mut packets = Vec::new();
                                for entry in replies {
                                    if accept {
                                        if let Err(e) = runtime.receive_from(
                                            config.members[entry.source_rank as usize]
                                                .certificate_hash,
                                            &entry.bytes,
                                        ) {
                                            result = Err(e.to_string());
                                        }
                                    }
                                    packets.push(entry.bytes);
                                }
                                if result.is_ok() {
                                    result = Ok(json!({"replies":packets}));
                                }
                            }
                        }
                    }
                }
            } else if target == rank {
                result = model::local(&command, &mut runtime, rank, size);
            }
            let response = match result {
                Ok(value) => json!({"ok":true,"result":value}),
                Err(error) => json!({"ok":false,"error":error}),
            };
            let responses = gather(&world, &response)?;
            if rank == 0 {
                println!("{}", json!({"responses":responses}));
                std::io::stdout().flush()?;
            }
        }
        exchange.barrier();
        drop(runtime);
        drop(exchange);
        drop(world);
        drop(universe);
        Ok(())
    }
}
#[cfg(all(unix, feature = "external-fault-injection"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    worker::run()
}
#[cfg(not(all(unix, feature = "external-fault-injection")))]
fn main() {
    eprintln!("Track49 MPI qualification requires Unix and external-fault-injection");
    std::process::exit(2);
}
