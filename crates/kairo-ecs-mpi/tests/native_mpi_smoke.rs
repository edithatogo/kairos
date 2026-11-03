// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "native-mpi")]
#![forbid(unsafe_code)]

use kairo_ecs_mpi::native::{
    NativeMpiExchange, NativeMpiInbound, NativeMpiRecord, MAX_NATIVE_RECORDS,
    MAX_NATIVE_RECORD_BATCH_BYTES, MAX_NATIVE_RECORD_BYTES,
};
use kairo_ecs_pdes::{LpId, NullMessage, PdesMessage, RemoteEvent, Tick};

// Run this binary through mpiexec -n 2 and -n 4, with --test-threads=1.
#[test]
fn native_mpi_exchange() {
    let universe = mpi::initialize().expect("this test owns MPI initialization");
    let exchange = NativeMpiExchange::new(&universe).unwrap();
    let rank = exchange.rank();
    let size = exchange.size();
    assert!([1, 2, 4].contains(&size));
    let destination = (rank + 1) % size;
    let tick = Tick::from_ticks((1u128 << 100) + rank as u128);
    let payload = vec![rank as u8; 4096];
    let input = vec![
        PdesMessage::Event(RemoteEvent {
            source_lp: LpId(rank),
            dest_lp: LpId(destination),
            tick,
            event_payload: payload,
        }),
        PdesMessage::Null(NullMessage {
            source_lp: LpId(rank),
            dest_lp: LpId(destination),
            safe_time: tick,
        }),
    ];
    let received = exchange.exchange(&input).unwrap();
    let predecessor = (rank + size - 1) % size;
    assert_eq!(
        received,
        vec![
            PdesMessage::Event(RemoteEvent {
                source_lp: LpId(predecessor),
                dest_lp: LpId(rank),
                tick: Tick::from_ticks((1u128 << 100) + predecessor as u128),
                event_payload: vec![predecessor as u8; 4096],
            }),
            PdesMessage::Null(NullMessage {
                source_lp: LpId(predecessor),
                dest_lp: LpId(rank),
                safe_time: Tick::from_ticks((1u128 << 100) + predecessor as u128),
            }),
        ]
    );
    assert_eq!(exchange.minimum(tick), Tick::from_ticks(1u128 << 100));
    // Invalid input on one rank must fail on every rank before the payload
    // collective, and the next valid collective must still work.
    let bad = if rank == 0 {
        vec![PdesMessage::Event(RemoteEvent {
            source_lp: LpId(size),
            dest_lp: LpId(0),
            tick: Tick::ZERO,
            event_payload: vec![],
        })]
    } else {
        vec![]
    };
    assert!(exchange.exchange(&bad).is_err());
    let oversized = if rank == 0 {
        vec![PdesMessage::Event(RemoteEvent {
            source_lp: LpId(0),
            dest_lp: LpId(0),
            tick: Tick::ZERO,
            event_payload: vec![0; 65536],
        })]
    } else {
        vec![]
    };
    assert!(exchange.exchange(&oversized).is_err());
    // Unequal batches and many-to-one delivery retain rank then FIFO order.
    let fan_in: Vec<_> = (0..rank)
        .map(|sequence| {
            PdesMessage::Event(RemoteEvent {
                source_lp: LpId(rank),
                dest_lp: LpId(0),
                tick: Tick::from_ticks(sequence as u128),
                event_payload: vec![rank as u8, sequence as u8],
            })
        })
        .collect();
    let fan_in_received = exchange.exchange(&fan_in).unwrap();
    let expected: Vec<_> = if rank == 0 {
        (0..size)
            .flat_map(|source| {
                (0..source).map(move |sequence| {
                    PdesMessage::Event(RemoteEvent {
                        source_lp: LpId(source),
                        dest_lp: LpId(0),
                        tick: Tick::from_ticks(sequence as u128),
                        event_payload: vec![source as u8, sequence as u8],
                    })
                })
            })
            .collect()
    } else {
        vec![]
    };
    assert_eq!(fan_in_received, expected);
    // Exactly 64 KiB (29-byte header + payload) succeeds; one extra byte fails.
    let boundary = vec![PdesMessage::Event(RemoteEvent {
        source_lp: LpId(rank),
        dest_lp: LpId(rank),
        tick,
        event_payload: vec![0x5a; 65536 - 29],
    })];
    assert_eq!(exchange.exchange(&boundary).unwrap(), boundary);
    let mut over_boundary = boundary;
    if rank == 0 {
        if let PdesMessage::Event(event) = &mut over_boundary[0] {
            event.event_payload.push(0);
        }
    }
    assert!(exchange.exchange(&over_boundary).is_err());
    assert!(exchange.exchange(&[]).unwrap().is_empty());
    // Byte-exact opaque transport carries signed external-driver records. The
    // actual MPI source slot remains independent of any in-band claimed issuer.
    let records = vec![NativeMpiRecord {
        destination,
        bytes: vec![rank as u8, 0, 255, 42],
    }];
    let mut validated = Vec::new();
    let incoming = exchange
        .exchange_records_validated(&records, |source, dest, bytes| {
            validated.push((source, dest));
            if bytes[0] as u32 != source {
                return Err(std::io::Error::other("issuer does not match MPI source"));
            }
            Ok(())
        })
        .unwrap();
    assert_eq!(
        validated,
        (0..size)
            .map(|source| (source, (source + 1) % size))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        incoming,
        vec![NativeMpiInbound {
            source_rank: predecessor,
            destination: rank,
            bytes: vec![predecessor as u8, 0, 255, 42],
        }]
    );
    // A policy rejection at just one validator rejects the entire round at
    // every rank before any result is returned, even when bytes are well formed.
    assert!(exchange
        .exchange_records_validated(&records, |_, _, _| {
            if rank == 0 {
                Err(std::io::Error::other("one validator rejects"))
            } else {
                Ok(())
            }
        })
        .is_err());
    assert_eq!(exchange.exchange_records(&records).unwrap(), incoming);
    let bad_records = if rank == 0 {
        vec![NativeMpiRecord {
            destination: size,
            bytes: vec![0],
        }]
    } else {
        vec![]
    };
    assert!(exchange.exchange_records(&bad_records).is_err());
    let oversized_record = if rank == 0 {
        vec![NativeMpiRecord {
            destination: 0,
            bytes: vec![0; MAX_NATIVE_RECORD_BYTES + 1],
        }]
    } else {
        vec![]
    };
    assert!(exchange.exchange_records(&oversized_record).is_err());
    let excess_count = if rank == 0 {
        vec![
            NativeMpiRecord {
                destination: 0,
                bytes: vec![0]
            };
            MAX_NATIVE_RECORDS + 1
        ]
    } else {
        vec![]
    };
    assert!(exchange.exchange_records(&excess_count).is_err());
    let spoofed_records = vec![NativeMpiRecord {
        destination,
        bytes: vec![if rank == 0 { 255 } else { rank as u8 }],
    }];
    assert!(exchange
        .exchange_records_validated(&spoofed_records, |source, _, bytes| {
            if bytes[0] as u32 == source {
                Ok(())
            } else {
                Err(std::io::Error::other("source spoof"))
            }
        })
        .is_err());
    let ordered: Vec<_> = (0..=rank)
        .map(|sequence| NativeMpiRecord {
            destination: 0,
            bytes: vec![rank as u8, sequence as u8],
        })
        .collect();
    let received = exchange.exchange_records(&ordered).unwrap();
    let expected: Vec<_> = if rank == 0 {
        (0..size)
            .flat_map(|source| {
                (0..=source).map(move |sequence| NativeMpiInbound {
                    source_rank: source,
                    destination: 0,
                    bytes: vec![source as u8, sequence as u8],
                })
            })
            .collect()
    } else {
        vec![]
    };
    assert_eq!(received, expected);
    let packet_boundary = vec![NativeMpiRecord {
        destination: rank,
        bytes: vec![0x5a; MAX_NATIVE_RECORD_BYTES],
    }];
    assert_eq!(
        exchange.exchange_records(&packet_boundary).unwrap()[0].bytes,
        packet_boundary[0].bytes
    );
    let mut batch_boundary = vec![
        NativeMpiRecord {
            destination: rank,
            bytes: vec![0x5a; MAX_NATIVE_RECORD_BYTES]
        };
        4
    ];
    batch_boundary[3]
        .bytes
        .truncate(MAX_NATIVE_RECORD_BYTES - 35);
    let received = exchange.exchange_records(&batch_boundary).unwrap();
    assert_eq!(
        received
            .iter()
            .map(|record| record.bytes.len())
            .sum::<usize>()
            + 35,
        MAX_NATIVE_RECORD_BATCH_BYTES
    );
    if rank == 0 {
        batch_boundary[3].bytes.push(0);
    }
    assert!(exchange.exchange_records(&batch_boundary).is_err());
    assert!(exchange.exchange_records(&[]).unwrap().is_empty());
    exchange.barrier();
    println!("native MPI API rank={rank} size={size}: ring, full-width minimum, opaque records, collective rejection and recovery passed");
}
