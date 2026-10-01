use kairo_ecs_gpu::{
    AgentParticle, CpuFallbackCompute, DesEvent, GpuCompute, GpuState, PersistentGpuSession,
    ResidentBufferKind,
};

fn mixed_state() -> GpuState {
    GpuState {
        particles: vec![
            AgentParticle {
                x: 1.0,
                y: 2.0,
                vx: 0.5,
                vy: -0.25,
            },
            AgentParticle {
                x: -4.0,
                y: 3.5,
                vx: 1.25,
                vy: 0.75,
            },
        ],
        entity_values: vec![0, 10, -2],
    }
}

fn mixed_events() -> Vec<DesEvent> {
    vec![
        DesEvent {
            timestamp_ns: 30,
            entity_id: 2,
            delta: 4,
        },
        DesEvent {
            timestamp_ns: 10,
            entity_id: 1,
            delta: -3,
        },
        DesEvent {
            timestamp_ns: 20,
            entity_id: 1,
            delta: 8,
        },
    ]
}

#[test]
fn persistent_session_keeps_state_buffers_resident_across_mixed_ticks() {
    let state = mixed_state();
    let events = mixed_events();
    let footprint = state.footprint().unwrap();

    let mut reference = CpuFallbackCompute::new();
    reference.upload_state(&state).unwrap();
    reference.run_abm_step(0.25, 99).unwrap();
    reference.run_des_step(&events).unwrap();

    let mut session = PersistentGpuSession::new("contract-gpu");
    let upload = session.upload_once(&state).unwrap();

    assert_eq!(upload.uploaded_bytes, footprint.total_bytes());
    assert_eq!(upload.downloaded_bytes, 0);
    assert!(session.is_resident(ResidentBufferKind::Particles));
    assert!(session.is_resident(ResidentBufferKind::EntityValues));

    let abm = session.run_abm_tick(0.25, 99).unwrap();
    let des = session.run_des_tick(&events).unwrap();
    let snapshot = session.residency_snapshot();

    assert_eq!(abm.uploaded_bytes, 0);
    assert_eq!(abm.downloaded_bytes, 0);
    assert_eq!(des.uploaded_bytes, core::mem::size_of_val(&events[..]));
    assert_eq!(des.downloaded_bytes, 0);
    assert_eq!(snapshot.resident_ticks, 2);
    assert_eq!(snapshot.host_state_uploads, 1);
    assert_eq!(snapshot.host_state_downloads, 0);
    assert_eq!(snapshot.state_bytes_resident, footprint.total_bytes());
    assert_eq!(
        session.download_state().unwrap(),
        reference.download_state().unwrap()
    );
    assert_eq!(session.residency_snapshot().host_state_downloads, 1);
}

#[test]
fn upload_once_reupload_is_counted_as_host_upload() {
    let state = mixed_state();
    let mut session = PersistentGpuSession::new("contract-gpu");

    session.upload_once(&state).unwrap();
    session.upload_once(&state).unwrap();

    let snapshot = session.residency_snapshot();
    assert_eq!(snapshot.host_state_uploads, 2);
}

#[test]
fn persistent_des_error_does_not_partially_apply_events() {
    let state = GpuState {
        particles: vec![],
        entity_values: vec![10, 20],
    };
    let events = vec![
        DesEvent {
            timestamp_ns: 10,
            entity_id: 0,
            delta: 5,
        },
        DesEvent {
            timestamp_ns: 20,
            entity_id: 99,
            delta: 1,
        },
    ];
    let mut session = PersistentGpuSession::new("contract-gpu");
    session.upload_once(&state).unwrap();

    assert_eq!(
        session.run_des_tick(&events),
        Err(kairo_ecs_gpu::GpuComputeError::EntityOutOfRange {
            entity_id: 99,
            entity_count: 2
        })
    );
    assert_eq!(session.download_state().unwrap(), state);
}
