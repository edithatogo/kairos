#![forbid(unsafe_code)]

use std::env;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use kairo_ecs_mpi::{MpiProcessTransport, MpiRankAssignment};
use kairo_ecs_pdes::{LpId, PdesMessage, PdesTransport, RemoteEvent, Tick};

#[test]
fn multirank_smoke_4ranks() {
    let mpi_env = MpiProcessTransport::detect_mpi_environment();

    if let Some((rank, size)) = mpi_env {
        // Run as an individual MPI rank
        assert_eq!(size, 4, "multirank_smoke expects exactly 4 ranks");
        run_mpi_rank_node(rank, size);
    } else {
        // Orchestrator parent: spawn 4 distinct OS processes running this test
        let current_exe = env::current_exe().expect("failed to determine current exe");
        let session_nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let session_id = format!("orchestrator_{}_{}", std::process::id(), session_nonce);

        let mut children = Vec::new();
        for rank in 0..4 {
            let child = Command::new(&current_exe)
                .arg("multirank_smoke_4ranks")
                .arg("--nocapture")
                .arg("--exact")
                .env("OMPI_COMM_WORLD_RANK", rank.to_string())
                .env("OMPI_COMM_WORLD_SIZE", "4")
                .env("KAIRO_MPI_SESSION_ID", &session_id)
                .spawn()
                .unwrap_or_else(|e| panic!("failed to spawn child rank {rank}: {e}"));
            children.push((rank, child));
        }

        for (rank, mut child) in children {
            let status = child
                .wait()
                .unwrap_or_else(|e| panic!("child rank {rank} failed to wait: {e}"));
            assert!(
                status.success(),
                "child rank {rank} exited with non-zero status: {status:?}"
            );
        }
    }
}

fn run_mpi_rank_node(rank: i32, size: usize) {
    let raw_session = env::var("KAIRO_MPI_SESSION_ID")
        .or_else(|_| env::var("PMIX_SERVER_TMPDIR"))
        .or_else(|_| env::var("PMIX_NAMESPACE"))
        .unwrap_or_else(|_| "kairo_multirank_smoke".to_string());
    let session_id = raw_session.replace(['/', '@', ':', '.', ' '], "_");

    let assignments = vec![
        MpiRankAssignment {
            rank: 0,
            lp_id: LpId(0),
        },
        MpiRankAssignment {
            rank: 1,
            lp_id: LpId(1),
        },
        MpiRankAssignment {
            rank: 2,
            lp_id: LpId(2),
        },
        MpiRankAssignment {
            rank: 3,
            lp_id: LpId(3),
        },
    ];

    let mut transport = MpiProcessTransport::new(rank, size, assignments, &session_id)
        .expect("failed to initialize MpiProcessTransport");

    assert!(transport.real_mpi_runtime_claimed());
    assert_eq!(transport.rank(), rank);
    assert_eq!(transport.size(), size);
    assert_eq!(transport.local_lp(), LpId(rank as u32));

    // Initial barrier to synchronize startup across all ranks
    transport.barrier();

    // 1. Ring event exchange: rank r sends event to rank (r + 1) % 4
    let target_rank = ((rank + 1) % (size as i32)) as u32;
    let target_lp = LpId(target_rank);
    let my_lp = LpId(rank as u32);

    let sent_event = RemoteEvent {
        source_lp: my_lp,
        dest_lp: target_lp,
        tick: Tick::from_ticks(100 + (rank as u128) * 10),
        event_payload: vec![rank as u8, 0xAA, 0xBB],
    };

    transport
        .send(target_lp, PdesMessage::Event(sent_event))
        .expect("send event must succeed");

    // Barrier to ensure all sends completed and delivered to inboxes
    transport.barrier();

    // 2. Receive and verify inbound event from predecessor in ring
    let expected_src_rank = ((rank + (size as i32) - 1) % (size as i32)) as u32;
    let messages = transport.recv(my_lp).expect("recv must succeed");
    assert_eq!(messages.len(), 1, "rank {rank} expected 1 message");

    match &messages[0] {
        PdesMessage::Event(e) => {
            assert_eq!(e.source_lp, LpId(expected_src_rank));
            assert_eq!(e.dest_lp, my_lp);
            assert_eq!(
                e.tick,
                Tick::from_ticks(100 + (expected_src_rank as u128) * 10)
            );
            assert_eq!(e.event_payload, vec![expected_src_rank as u8, 0xAA, 0xBB]);
        }
        PdesMessage::Null(_) => panic!("expected Event message, got Null"),
    }

    // 3. Collective all-reduce min for GVT
    // Rank 0: 250, Rank 1: 150, Rank 2: 100 (min), Rank 3: 300
    let my_proposal = match rank {
        0 => Tick::from_ticks(250),
        1 => Tick::from_ticks(150),
        2 => Tick::from_ticks(100),
        3 => Tick::from_ticks(300),
        _ => Tick::from_ticks(999),
    };

    let global_min = transport.all_reduce_min(my_proposal);
    assert_eq!(
        global_min,
        Tick::from_ticks(100),
        "rank {rank} calculated wrong all_reduce_min: {global_min:?}"
    );

    // Final barrier before clean shutdown
    transport.barrier();
}
