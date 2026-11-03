// SPDX-License-Identifier: Apache-2.0
//! Synthetic entity migration through the authenticated, durable external engine.
#[cfg(all(unix, feature = "external-fault-injection"))]
#[path = "../../../docs/distributed/track49-fixtures/model.rs"]
mod model;
#[cfg(all(unix, feature = "external-fault-injection"))]
mod worker {
    use super::model;
    use kairo_ecs_grpc::{
        external_exchange, serve_external_tls, ExternalDispatcher, ExternalTlsIdentity,
        ExternalTlsPeer,
    };
    use kairo_ecs_pdes::external::protocol::Hash;
    use kairo_ecs_pdes::external::ExternalConfiguration;
    use serde::Deserialize;
    use serde_json::{json, Value};
    use std::collections::{BTreeMap, BTreeSet};
    use std::io::{BufRead, Write};
    use std::path::Path;
    use std::sync::{Arc, Mutex};
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Settings {
        configuration: ExternalConfiguration,
        participant: u32,
        store: String,
        listen: String,
        certificate: String,
        private_key: String,
        client_ca: String,
        peers: Vec<Peer>,
    }
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Peer {
        participant: u32,
        endpoint: String,
        domain: String,
        ca: String,
        certificate_hash: Hash,
    }
    pub async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let arg = std::env::args().nth(1).ok_or("settings path required")?;
        if arg == "--describe" {
            println!("{}", model::describe(2));
            return Ok(());
        }
        if arg == "--serial" {
            println!("{}", model::serial(2)?);
            return Ok(());
        }
        let raw = std::fs::read(&arg)?;
        if raw.len() > 65536 {
            return Err("settings bound".into());
        }
        let settings: Settings = serde_json::from_slice(&raw)?;
        // This owned executable selects its provider before constructing TLS.
        rustls::crypto::ring::default_provider()
            .install_default()
            .map_err(|_| "unexpected prior TLS provider")?;
        let identity = ExternalTlsIdentity {
            certificate_pem: std::fs::read(&settings.certificate)?,
            private_key_pem: std::fs::read(&settings.private_key)?,
            client_ca_pem: std::fs::read(&settings.client_ca)?,
        };
        let mut peers = BTreeMap::new();
        for p in settings.peers {
            if peers
                .insert(
                    p.participant,
                    ExternalTlsPeer {
                        endpoint: p.endpoint,
                        domain: p.domain,
                        ca_pem: std::fs::read(p.ca)?,
                        certificate_hash: p.certificate_hash,
                    },
                )
                .is_some()
            {
                return Err("duplicate peer".into());
            }
        }
        let allowed = settings
            .configuration
            .members
            .iter()
            .map(|m| m.certificate_hash)
            .collect::<BTreeSet<_>>();
        let runtime = Arc::new(Mutex::new(model::open(
            Path::new(&settings.store),
            settings.configuration.clone(),
            settings.participant,
        )?));
        let listener = tokio::net::TcpListener::bind(&settings.listen).await?;
        let address = listener.local_addr()?;
        let driver = Arc::clone(&runtime);
        let dispatcher: ExternalDispatcher = Arc::new(move |leaf, packet| {
            driver
                .lock()
                .map_err(|_| "driver poisoned".to_string())?
                .receive_from(leaf, &packet)
                .map_err(|e| e.to_string())
        });
        let (tx, rx) = tokio::sync::oneshot::channel();
        let server_id = identity.clone();
        let server = tokio::spawn(async move {
            serve_external_tls(listener, server_id, allowed, dispatcher, rx).await
        });
        println!(
            "{}",
            json!({"ready":true,"pid":std::process::id(),"listen":address,"generation":runtime.lock().unwrap().generation()})
        );
        std::io::stdout().flush()?;
        for line in std::io::stdin().lock().lines() {
            let raw = line?;
            if raw.len() > 1024 * 1024 {
                return Err("owned command bound".into());
            }
            let v: Value = serde_json::from_str(&raw)?;
            let op = v.get("op").and_then(Value::as_str).ok_or("op")?;
            if op == "stop" {
                break;
            }
            let result: Result<Value, String> = if op == "send" {
                let destination: u32 = model::field(&v, "destination")?;
                let packet: Vec<u8> = model::field(&v, "packet")?;
                match external_exchange(&identity, peers.get(&destination).ok_or("peer")?, packet)
                    .await
                {
                    Err(e) => Err(format!("{e:?}")),
                    Ok(replies) => {
                        let mut result = Ok(json!({"replies":replies}));
                        if v.get("accept").and_then(Value::as_bool).unwrap_or(true) {
                            for packet in &replies {
                                if let Err(e) = runtime.lock().unwrap().receive_from(
                                    settings.configuration.members[destination as usize]
                                        .certificate_hash,
                                    packet,
                                ) {
                                    result = Err(e.to_string());
                                }
                            }
                        }
                        result
                    }
                }
            } else {
                let mut guard = runtime.lock().map_err(|_| "driver poisoned")?;
                model::local(
                    &v,
                    &mut guard,
                    settings.participant,
                    settings.configuration.members.len() as u32,
                )
            };
            let response = match result {
                Ok(result) => json!({"ok":true,"result":result}),
                Err(error) => json!({"ok":false,"error":error}),
            };
            println!("{response}");
            std::io::stdout().flush()?;
        }
        let _ = tx.send(());
        server.await??;
        Ok(())
    }
}
#[cfg(all(unix, feature = "external-fault-injection"))]
#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    worker::run().await
}
#[cfg(not(all(unix, feature = "external-fault-injection")))]
fn main() {
    eprintln!("Track49 qualification requires Unix and external-fault-injection");
    std::process::exit(2);
}
