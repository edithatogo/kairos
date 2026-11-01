use kairo_ecs_grpc::{GrpcTransportConfig, TonicTransport};
use kairo_ecs_pdes::LpId;
use std::{
    thread,
    time::{Duration, Instant},
};
fn main() {
    let server = TonicTransport::bind(
        LpId(1),
        "127.0.0.1:0".parse().unwrap(),
        vec![],
        GrpcTransportConfig::default(),
    )
    .unwrap();
    println!("{}", server.bound_addr().port());
    let deadline = Instant::now() + Duration::from_secs(60);
    while Instant::now() < deadline {
        let received = server.drain_heartbeats();
        if !received.is_empty() {
            assert_eq!(received.len(), 1);
            assert_eq!(received[0].lp_id, LpId(99));
            assert_eq!(
                received[0].elapsed_since_last_seen,
                Duration::from_millis(15)
            );
            println!("received LP 99, elapsed 15 ms");
            thread::sleep(Duration::from_millis(100));
            return;
        }
        thread::sleep(Duration::from_millis(10));
    }
    panic!("missing NAS heartbeat");
}
