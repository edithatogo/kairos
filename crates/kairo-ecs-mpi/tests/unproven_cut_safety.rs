// SPDX-License-Identifier: Apache-2.0
#![forbid(unsafe_code)]

use kairo_ecs_mpi::{MpiProcessTransport, MpiRankAssignment};
use kairo_ecs_pdes::{LpId, PdesTransport, Tick};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn responsive_launcher_tcp_ranks_do_not_certify_a_distributed_cut() {
    let session = format!(
        "unproven_cut_{}_{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let assignment = vec![
        MpiRankAssignment {
            rank: 0,
            lp_id: LpId(0),
        },
        MpiRankAssignment {
            rank: 1,
            lp_id: LpId(1),
        },
    ];
    let mut first = MpiProcessTransport::new(0, 2, assignment.clone(), &session).unwrap();
    let mut second = MpiProcessTransport::new(1, 2, assignment, &session).unwrap();
    let peer = thread::spawn(move || {
        let diagnostic = second.reduce_reported_minimum(Tick::from_ticks(300));
        (diagnostic, second.all_reduce_min(Tick::from_ticks(300)))
    });
    assert_eq!(
        first.reduce_reported_minimum(Tick::from_ticks(500)),
        Tick::from_ticks(300),
        "same participants must first complete a positive diagnostic round"
    );
    assert_eq!(first.all_reduce_min(Tick::from_ticks(500)), Tick::ZERO);
    assert_eq!(peer.join().unwrap(), (Tick::from_ticks(300), Tick::ZERO));
}

#[test]
fn one_launcher_tcp_rank_is_still_an_open_network_listener() {
    let session = format!(
        "unproven_single_{}_{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let mut transport = MpiProcessTransport::new(
        0,
        1,
        vec![MpiRankAssignment {
            rank: 0,
            lp_id: LpId(0),
        }],
        &session,
    )
    .unwrap();
    assert_eq!(transport.all_reduce_min(Tick::from_ticks(500)), Tick::ZERO);
    assert_eq!(
        transport.reduce_reported_minimum(Tick::from_ticks(500)),
        Tick::from_ticks(500)
    );
}
