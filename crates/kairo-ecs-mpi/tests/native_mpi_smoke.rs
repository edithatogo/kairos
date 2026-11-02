// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "native-mpi")]
#![forbid(unsafe_code)]

use kairo_ecs_mpi::native::NativeMpiExchange;
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
    exchange.barrier();
    println!("native MPI API rank={rank} size={size}: ring, full-width minimum, rejection and recovery passed");
}
