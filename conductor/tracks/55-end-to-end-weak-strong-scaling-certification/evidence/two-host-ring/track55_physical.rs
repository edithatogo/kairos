// SPDX-License-Identifier: Apache-2.0
//! Track 55 native ring transport experiment. The existing benchmark supplies the
//! model, deterministic tape, and serial oracle.
fn main() {
    physical::run();
}

#[allow(dead_code, unexpected_cfgs)]
mod physical {
    include!("../../kairo-ecs-pdes/benches/production.rs");

    use kairo_ecs_grpc::{GrpcPeer, GrpcSocketTransport, GrpcTransportConfig};
    use kairo_ecs_pdes::{PdesMessage, PdesTransport};
    use serde::{Deserialize, Serialize};
    use std::{net::SocketAddr, thread, time::Duration};

    const CONTROLLER: LpId = LpId(2);
    const RING: u32 = 2;
    const EVENT_WORK: u32 = 8_192;
    const ROOTS_PER_HOST: usize = 1_024;
    const TRIALS: usize = 10;
    type Batches = [Vec<u64>; RING as usize];
    type FinalState = [[u64; 2]; RING as usize];

    #[derive(Serialize, Deserialize)]
    enum Command {
        Dispatch {
            sample: usize,
            tick: u8,
            batches: Batches,
        },
        Stop,
    }

    #[derive(Serialize, Deserialize)]
    struct Reply {
        sample: usize,
        tick: u8,
        emitted: Batches,
        state: FinalState,
        worker_dispatch_ns: u128,
    }

    fn bind(local: LpId, listen: &str, peers: &[String]) -> GrpcSocketTransport {
        let peers = peers
            .iter()
            .map(|peer| {
                let (lp, address) = peer.split_once('=').unwrap();
                let address = address.parse::<SocketAddr>().unwrap();
                assert!(address.ip().is_loopback());
                GrpcPeer {
                    lp_id: LpId(lp.parse().unwrap()),
                    endpoint: format!("http://{address}"),
                }
            })
            .collect();
        let listen = listen.parse::<SocketAddr>().unwrap();
        assert!(listen.ip().is_loopback());
        GrpcSocketTransport::bind(local, listen, peers, GrpcTransportConfig::default()).unwrap()
    }

    fn send<T: Serialize>(transport: &mut GrpcSocketTransport, dest: LpId, body: &T) {
        let message = PdesMessage::Event(RemoteEvent {
            source_lp: transport.local_lp(),
            dest_lp: dest,
            // Outer frames transfer batches; the logical tick is in each command.
            tick: SimTime::ZERO,
            event_payload: serde_json::to_vec(body).unwrap(),
        });
        transport.send(dest, message).unwrap();
    }

    fn receive(transport: &mut GrpcSocketTransport, count: usize) -> Vec<RemoteEvent> {
        let deadline = Instant::now() + Duration::from_secs(15);
        let local = transport.local_lp();
        let mut events = Vec::with_capacity(count);
        while events.len() < count {
            assert!(
                Instant::now() < deadline,
                "peer reply deadline at {local:?}"
            );
            for message in transport.recv(local).unwrap() {
                let PdesMessage::Event(event) = message else {
                    panic!("unexpected non-event frame");
                };
                assert_eq!(event.dest_lp, local);
                events.push(event);
            }
            if events.len() < count {
                thread::sleep(Duration::from_micros(100));
            }
        }
        assert_eq!(events.len(), count);
        events
    }

    fn dispatch(
        processes: &mut [WorkloadProcess; RING as usize],
        tick: u8,
        batches: Batches,
    ) -> Batches {
        let simulated_time = SimTime::from_ticks(u128::from(tick));
        let mut scheduler = Scheduler::new();
        for (lp, values) in batches.into_iter().enumerate() {
            for value in values {
                scheduler.schedule(ScheduleRequest {
                    at: simulated_time,
                    priority: 0,
                    entity: Some(EntityId::new(value, lp as u32)),
                    kind: EventKind::custom(tick.into()),
                });
            }
        }
        let mut emitted = Batches::default();
        while let StepOutcome::Dispatched(event) = scheduler.step() {
            let entity = event.entity.unwrap();
            let dest = entity.generation;
            let source = if tick == 0 {
                dest
            } else {
                (dest + RING - 1) % RING
            };
            let native_event = RemoteEvent {
                source_lp: LpId(source),
                dest_lp: LpId(dest),
                tick: event.at,
                event_payload: encode_payload(tick, entity.index),
            };
            for outbound in processes[dest as usize].on_event(&native_event) {
                assert_eq!(outbound.tick, SimTime::from_ticks(1));
                emitted[outbound.dest_lp.0 as usize].push(payload_value(&outbound));
            }
        }
        emitted
    }

    fn worker(local: LpId, listen: &str, peer: String) {
        let mut transport = bind(local, listen, &[peer]);
        println!("ready {}", local.0);
        let mut processes = std::array::from_fn(|_| WorkloadProcess::new(RING, EVENT_WORK));
        let mut expected_followup = None;
        loop {
            let event = receive(&mut transport, 1).pop().unwrap();
            assert_eq!(event.source_lp, CONTROLLER);
            let cmd = serde_json::from_slice::<Command>(&event.event_payload).unwrap();
            let Command::Dispatch {
                sample,
                tick,
                batches,
            } = cmd
            else {
                assert!(expected_followup.is_none());
                return;
            };
            if tick == 0 {
                assert!(expected_followup.replace(sample).is_none());
                processes = std::array::from_fn(|_| WorkloadProcess::new(RING, EVENT_WORK));
            } else {
                assert_eq!(tick, 1);
                assert_eq!(expected_followup.take(), Some(sample));
            }
            let started = Instant::now();
            let emitted = dispatch(&mut processes, tick, batches);
            let worker_dispatch_ns = started.elapsed().as_nanos();
            let state = processes
                .each_ref()
                .map(|lp| [lp.state.processed, lp.state.checksum]);
            send(
                &mut transport,
                CONTROLLER,
                &Reply {
                    sample,
                    tick,
                    emitted,
                    state,
                    worker_dispatch_ns,
                },
            );
        }
    }

    fn phase(
        transport: &mut GrpcSocketTransport,
        hosts: usize,
        sample: usize,
        tick: u8,
        input: &Batches,
    ) -> (Batches, FinalState, Vec<u128>) {
        for host in 0..hosts {
            let batches = std::array::from_fn(|lp| {
                if lp % hosts == host {
                    input[lp].clone()
                } else {
                    Vec::new()
                }
            });
            send(
                transport,
                LpId(host as u32),
                &Command::Dispatch {
                    sample,
                    tick,
                    batches,
                },
            );
        }
        let mut emitted = Batches::default();
        let mut state = FinalState::default();
        let mut dispatch_times = vec![0; hosts];
        for event in receive(transport, hosts) {
            let host = event.source_lp.0 as usize;
            assert!(host < hosts);
            assert_eq!(dispatch_times[host], 0);
            let result = serde_json::from_slice::<Reply>(&event.event_payload).unwrap();
            assert_eq!((result.sample, result.tick), (sample, tick));
            assert!(result.worker_dispatch_ns > 0);
            dispatch_times[host] = result.worker_dispatch_ns;
            for (lp, next) in result.emitted.into_iter().enumerate() {
                if lp % hosts == host {
                    state[lp] = result.state[lp];
                } else {
                    assert_eq!(result.state[lp], [0, 0]);
                }
                if !next.is_empty() {
                    assert_eq!(((lp + RING as usize - 1) % RING as usize) % hosts, host);
                    assert!(emitted[lp].is_empty());
                    emitted[lp] = next;
                }
            }
        }
        (emitted, state, dispatch_times)
    }

    struct Tape {
        inputs: Batches,
        expected: FinalState,
        sha256: String,
        serial_dispatch_ns: u128,
    }

    fn tape(count: usize) -> Tape {
        let events = make_inputs(DEFAULT_SEED, RING, count);
        let mut inputs = Batches::default();
        for event in &events {
            inputs[event.dest_lp.0 as usize].push(payload_value(event));
        }
        let (serial, serial_times) = run_sequential(RING, &events, EVENT_WORK);
        let expected = std::array::from_fn(|i| [serial[i].processed, serial[i].checksum]);
        Tape {
            inputs,
            expected,
            sha256: tape_sha256(&events),
            serial_dispatch_ns: serial_times.dispatch_ns,
        }
    }

    fn controller(listen: &str, peers: &[String]) {
        assert_eq!(peers.len(), 2);
        let mut transport = bind(CONTROLLER, listen, peers);
        let base = tape(ROOTS_PER_HOST);
        let doubled = tape(ROOTS_PER_HOST * 2);
        let mut sample = 0;
        for trial in 0..=TRIALS {
            for mode in ["strong", "weak"] {
                for hosts in if trial % 2 == 0 { [1, 2] } else { [2, 1] } {
                    let tape = if mode == "weak" && hosts == 1 {
                        &base
                    } else {
                        &doubled
                    };
                    let started = Instant::now();
                    let (followups, _, tick0_ns) =
                        phase(&mut transport, hosts, sample, 0, &tape.inputs);
                    let (remaining, actual, tick1_ns) =
                        phase(&mut transport, hosts, sample, 1, &followups);
                    let wall_ns = started.elapsed().as_nanos();
                    assert!(remaining.iter().all(Vec::is_empty));
                    assert_eq!(followups[0], tape.inputs[1]);
                    assert_eq!(followups[1], tape.inputs[0]);
                    assert_eq!(actual, tape.expected);
                    println!(
                        "{}",
                        json!({
                            "sample": sample, "trial": trial, "warmup": trial == 0, "mode": mode,
                            "physical_hosts": hosts, "logical_lps": RING, "input_seed": DEFAULT_SEED,
                            "event_work_mixes": EVENT_WORK, "input_sha256": tape.sha256,
                            "root_events": tape.inputs.iter().map(Vec::len).sum::<usize>(),
                            "events": actual.iter().map(|state| state[0]).sum::<u64>(),
                            "state": actual, "serial_expected": tape.expected, "parity": true,
                            "native_frames": hosts * 4, "cross_host_frames": (hosts - 1) * 4,
                            "wall_ns": wall_ns, "serial_oracle_fixture_ns": tape.serial_dispatch_ns,
                            "tick0_worker_dispatch_ns": tick0_ns, "tick1_worker_dispatch_ns": tick1_ns,
                        })
                    );
                    sample += 1;
                }
            }
        }
        for host in 0..peers.len() {
            send(&mut transport, LpId(host as u32), &Command::Stop);
        }
        assert_eq!(transport.envelope_decode_failure_count(), 0);
    }

    pub fn run() {
        let args = std::env::args().skip(1).collect::<Vec<_>>();
        match args.as_slice() {
            [mode, id, listen, peer] if mode == "worker" => {
                worker(LpId(id.parse().unwrap()), listen, peer.clone())
            }
            [mode, listen, peers @ ..] if mode == "controller" => controller(listen, peers),
            _ => panic!("usage: track55_physical worker ID ADDRESS PEER_ID=ADDRESS | controller ADDRESS PEER_ID=ADDRESS PEER_ID=ADDRESS"),
        }
    }
}
