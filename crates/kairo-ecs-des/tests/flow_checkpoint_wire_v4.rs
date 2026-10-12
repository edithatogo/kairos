use kairo_ecs_des::{
    FlowAllocationMode, FlowCheckpointCodecError, FlowCheckpointCodecs, FlowCheckpointLimits,
    FlowCheckpointWireError, FlowCheckpointWireLimits, FlowQueueOrderKey, FlowQueueOrdering,
    FlowRequestQueueOrderV4, FlowRuntime, RequestState, ResourceId, FLOW_CHECKPOINT_VERSION_V4,
};
use kairo_ecs_types::{SimDuration, SimTime};

type MixedSeed = (
    FlowRuntime,
    FlowCheckpointCodecs,
    ResourceId,
    ResourceId,
    kairo_ecs_des::RequestId,
    kairo_ecs_des::RequestId,
    kairo_ecs_des::RequestId,
    kairo_ecs_des::RequestId,
    kairo_ecs_des::RequestId,
);

fn at(tick: u128) -> SimTime {
    SimTime::from_ticks(tick)
}

fn codecs() -> FlowCheckpointCodecs {
    fn encode(value: &u64, remaining: usize) -> Result<Vec<u8>, FlowCheckpointCodecError> {
        if remaining < 8 {
            return Err(FlowCheckpointCodecError("payload limit".into()));
        }
        Ok(value.to_le_bytes().to_vec())
    }
    fn decode(
        bytes: &[u8],
        _: &kairo_ecs_des::FlowCheckpointRebindV1,
    ) -> Result<u64, FlowCheckpointCodecError> {
        Ok(u64::from_le_bytes(bytes.try_into().map_err(|_| {
            FlowCheckpointCodecError("bad u64 payload".into())
        })?))
    }
    let mut result = FlowCheckpointCodecs::new();
    result.register_context("ctx", 1, encode, decode).unwrap();
    result
}

fn mixed_seed() -> MixedSeed {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let stable = flow
        .create_resource_with_queue_ordering(
            1,
            FlowAllocationMode::Eager,
            FlowQueueOrdering::StableKey,
        )
        .unwrap();
    let admission = flow
        .create_resource_with_allocation_mode(1, FlowAllocationMode::Deferred)
        .unwrap();
    let work = flow
        .create_work(owner, SimDuration::from_ticks(20), "ctx", 9_u64)
        .unwrap();
    let active = flow
        .acquire(stable)
        .owner(owner)
        .timed_work(work)
        .preemptible(kairo_ecs_des::PreemptionStrategy::Suspend)
        .queue_order_key(FlowQueueOrderKey::new(b"active").unwrap())
        .submit()
        .unwrap();
    flow.step().unwrap().unwrap();
    let urgent_work = flow
        .create_work(owner, SimDuration::from_ticks(3), "ctx", 10_u64)
        .unwrap();
    flow.acquire(stable)
        .owner(owner)
        .at(at(1))
        .timed_work(urgent_work)
        .priority(-1)
        .can_preempt(true)
        .queue_order_key(FlowQueueOrderKey::new(b"urgent").unwrap())
        .submit()
        .unwrap();
    flow.step().unwrap().unwrap();
    let queued = flow
        .acquire(stable)
        .owner(owner)
        .queue_order_key(FlowQueueOrderKey::new(b"same").unwrap())
        .submit()
        .unwrap();
    flow.step().unwrap().unwrap();
    let duplicate = flow
        .acquire(stable)
        .owner(owner)
        .queue_order_key(FlowQueueOrderKey::new(b"same").unwrap())
        .submit()
        .unwrap();
    flow.step().unwrap().unwrap();
    let admission_active = flow.acquire(admission).owner(owner).submit().unwrap();
    flow.step().unwrap().unwrap();
    let pending = flow
        .acquire(stable)
        .owner(owner)
        .at(at(10))
        .queue_order_key(FlowQueueOrderKey::new(b"future").unwrap())
        .submit()
        .unwrap();
    (
        flow,
        codecs(),
        stable,
        admission,
        active,
        queued,
        duplicate,
        pending,
        admission_active,
    )
}

fn wire_limits() -> FlowCheckpointWireLimits {
    FlowCheckpointWireLimits::default()
}

fn dto_string_key_bytes(image: &kairo_ecs_des::FlowCheckpointV4) -> usize {
    let work = image.work_registrations.iter().fold(0, |sum, row| {
        sum + row.context_runtime_key.len()
            + row.context_codec_key.len()
            + row.restart_codec_key.as_ref().map_or(0, String::len)
    });
    let contexts = image
        .contexts
        .iter()
        .map(|row| row.runtime_key.len() + row.codec_key.len())
        .sum::<usize>();
    let callbacks = image
        .handlers
        .iter()
        .chain(&image.continuations)
        .chain(&image.domains)
        .map(|row| {
            row.runtime_key.len()
                + row.context_codec_key.len()
                + row
                    .callback_ids
                    .iter()
                    .map(|(name, code)| name.len() + code.stable_id.len())
                    .sum::<usize>()
        })
        .sum::<usize>();
    let context_stores = image
        .context_stores
        .iter()
        .map(|row| row.codec_key.len())
        .sum::<usize>();
    let manifest = image
        .restart_store_manifest
        .iter()
        .map(|(key, _)| key.len())
        .sum::<usize>();
    let restart_stores = image
        .restart_stores
        .iter()
        .map(|row| {
            row.codec_key.len()
                + row
                    .rows
                    .iter()
                    .map(|(_, factory, _)| factory.len())
                    .sum::<usize>()
        })
        .sum::<usize>();
    let work_specs = image.builtins.work_specs.as_ref().map_or(0, |store| {
        store
            .rows
            .iter()
            .map(|(_, spec)| spec.context_type_key.len())
            .sum()
    });
    work + contexts + callbacks + context_stores + manifest + restart_stores + work_specs
}

fn one_stable_key_wire(key: &[u8]) -> Vec<u8> {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let resource = flow
        .create_resource_with_queue_ordering(
            0,
            FlowAllocationMode::Eager,
            FlowQueueOrdering::StableKey,
        )
        .unwrap();
    flow.acquire(resource)
        .owner(owner)
        .queue_order_key(FlowQueueOrderKey::new(key).unwrap())
        .submit()
        .unwrap();
    flow.capture_checkpoint_v4(
        &FlowCheckpointCodecs::new(),
        FlowCheckpointLimits::default(),
    )
    .unwrap()
    .encode_wire_v4(wire_limits())
    .unwrap()
}

fn two_equal_length_request_keys_wire(key: &[u8]) -> Vec<u8> {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let resource = flow
        .create_resource_with_queue_ordering(
            0,
            FlowAllocationMode::Eager,
            FlowQueueOrdering::StableKey,
        )
        .unwrap();
    for _ in 0..2 {
        flow.acquire(resource)
            .owner(owner)
            .queue_order_key(FlowQueueOrderKey::new(key).unwrap())
            .submit()
            .unwrap();
    }
    flow.capture_checkpoint_v4(
        &FlowCheckpointCodecs::new(),
        FlowCheckpointLimits::default(),
    )
    .unwrap()
    .encode_wire_v4(wire_limits())
    .unwrap()
}

fn two_resource_modes_wire() -> Vec<u8> {
    let mut flow = FlowRuntime::new();
    flow.create_resource_with_queue_ordering(
        0,
        FlowAllocationMode::Eager,
        FlowQueueOrdering::StableKey,
    )
    .unwrap();
    flow.create_resource(0).unwrap();
    flow.capture_checkpoint_v4(
        &FlowCheckpointCodecs::new(),
        FlowCheckpointLimits::default(),
    )
    .unwrap()
    .encode_wire_v4(wire_limits())
    .unwrap()
}

fn foreign_request_id() -> kairo_ecs_des::RequestId {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let resource = flow.create_resource(0).unwrap();
    let mut last = None;
    for _ in 0..16 {
        last = Some(flow.acquire(resource).owner(owner).submit().unwrap());
    }
    last.unwrap()
}

fn one_key_with_context_image() -> (kairo_ecs_des::FlowCheckpointV4, FlowCheckpointCodecs) {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let resource = flow
        .create_resource_with_queue_ordering(
            0,
            FlowAllocationMode::Eager,
            FlowQueueOrdering::StableKey,
        )
        .unwrap();
    let work = flow
        .create_work(owner, SimDuration::from_ticks(5), "ctx", 42_u64)
        .unwrap();
    flow.acquire(resource)
        .owner(owner)
        .timed_work(work)
        .queue_order_key(FlowQueueOrderKey::new(&[b'k'; 128]).unwrap())
        .submit()
        .unwrap();
    let registry = codecs();
    let image = flow
        .capture_checkpoint_v4(&registry, FlowCheckpointLimits::default())
        .unwrap();
    (image, registry)
}

fn roundtrip(image: &kairo_ecs_des::FlowCheckpointV4) -> Vec<u8> {
    let bytes = image.encode_wire_v4(wire_limits()).unwrap();
    assert_eq!(
        kairo_ecs_des::FlowCheckpointV4::decode_wire_v4(&bytes, wire_limits()).unwrap(),
        image.clone()
    );
    bytes
}

#[test]
fn v4_wire_actual_mixed_runtime_roundtrip_continues() {
    let (source, registry, stable, admission, active, queued, duplicate, pending, deferred_request) =
        mixed_seed();
    let mut source = source;
    let image = source
        .capture_checkpoint_v4(&registry, FlowCheckpointLimits::default())
        .unwrap();
    assert!(image
        .allocation_modes
        .iter()
        .any(|(_, mode)| *mode == FlowAllocationMode::Eager));
    assert!(image
        .allocation_modes
        .iter()
        .any(|(id, mode)| *id == stable && *mode == FlowAllocationMode::Eager));
    assert!(image
        .allocation_modes
        .iter()
        .any(|(id, mode)| *id == admission && *mode == FlowAllocationMode::Deferred));
    assert!(
        image
            .request_queue_orders
            .iter()
            .any(|(id, order)| *id == queued
                && matches!(order, FlowRequestQueueOrderV4::StableKey(_)))
    );
    assert!(image
        .request_queue_orders
        .iter()
        .any(|(id, order)| *id == duplicate
            && matches!(order, FlowRequestQueueOrderV4::StableKey(_))));
    assert!(image.request_queue_orders.iter().any(
        |(id, order)| *id == pending && matches!(order, FlowRequestQueueOrderV4::StableKey(_))
    ));
    assert!(image
        .builtins
        .requests
        .as_ref()
        .unwrap()
        .rows
        .iter()
        .any(|(_, request)| request.state == RequestState::Suspended));
    assert_eq!(
        image
            .builtins
            .requests
            .as_ref()
            .unwrap()
            .rows
            .iter()
            .find(|(id, _)| *id == pending.entity_id())
            .unwrap()
            .1
            .state,
        RequestState::Pending
    );
    assert!(image
        .context_stores
        .iter()
        .any(|store| !store.rows.is_empty()));
    assert!(matches!(
        image.encode_wire_v4(FlowCheckpointWireLimits {
            flow: FlowCheckpointLimits {
                max_payload_bytes: 0,
                ..FlowCheckpointLimits::default()
            },
            ..wire_limits()
        }),
        Err(FlowCheckpointWireError::LimitExceeded(
            "aggregate payload byte"
        ))
    ));
    let payload_bytes = image
        .context_stores
        .iter()
        .flat_map(|store| &store.rows)
        .map(|(_, payload)| payload.len())
        .chain(
            image
                .restart_stores
                .iter()
                .flat_map(|store| &store.rows)
                .map(|(_, _, payload)| payload.len()),
        )
        .sum::<usize>();
    assert!(image
        .encode_wire_v4(FlowCheckpointWireLimits {
            flow: FlowCheckpointLimits {
                max_payload_bytes: payload_bytes,
                ..FlowCheckpointLimits::default()
            },
            ..wire_limits()
        })
        .is_ok());
    assert!(image
        .encode_wire_v4(FlowCheckpointWireLimits {
            flow: FlowCheckpointLimits {
                max_payload_bytes: payload_bytes - 1,
                ..FlowCheckpointLimits::default()
            },
            ..wire_limits()
        })
        .is_err());
    let bytes = roundtrip(&image);
    assert!(kairo_ecs_des::FlowCheckpointV4::decode_wire_v4(
        &bytes,
        FlowCheckpointWireLimits {
            flow: FlowCheckpointLimits {
                max_requests: image.requests.len() - 1,
                ..FlowCheckpointLimits::default()
            },
            ..wire_limits()
        }
    )
    .is_err());
    let decoded = kairo_ecs_des::FlowCheckpointV4::decode_wire_v4(&bytes, wire_limits()).unwrap();
    let mut restored =
        FlowRuntime::restore_checkpoint_v4(decoded, &registry, FlowCheckpointLimits::default())
            .unwrap();
    assert_eq!(
        restored.request(active).unwrap(),
        source.request(active).unwrap()
    );
    assert_eq!(
        restored.request(deferred_request).unwrap(),
        source.request(deferred_request).unwrap()
    );
    assert_eq!(
        restored.request(queued).unwrap().state,
        RequestState::Queued
    );
    assert_eq!(
        restored.resource_queue_ordering(stable).unwrap(),
        FlowQueueOrdering::StableKey
    );
    for _ in 0..16 {
        let left = source.step().unwrap();
        let right = restored.step().unwrap();
        assert_eq!(left, right);
        if left.is_none() {
            break;
        }
    }
    assert_eq!(
        source
            .capture_checkpoint_v4(&registry, FlowCheckpointLimits::default())
            .unwrap(),
        restored
            .capture_checkpoint_v4(&registry, FlowCheckpointLimits::default())
            .unwrap()
    );
}

#[test]
fn v4_wire_removed_generation_terminal_rows_roundtrip() {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let removed = flow
        .create_resource_with_queue_ordering(
            0,
            FlowAllocationMode::Eager,
            FlowQueueOrdering::StableKey,
        )
        .unwrap();
    let request = flow
        .acquire(removed)
        .owner(owner)
        .queue_order_key(FlowQueueOrderKey::new(b"terminal").unwrap())
        .submit()
        .unwrap();
    flow.step().unwrap().unwrap();
    let second_request = flow
        .acquire(removed)
        .owner(owner)
        .queue_order_key(FlowQueueOrderKey::new(b"terminal-2").unwrap())
        .submit()
        .unwrap();
    flow.step().unwrap().unwrap();
    flow.cancel(request, at(1)).unwrap();
    flow.cancel(second_request, at(1)).unwrap();
    flow.step().unwrap().unwrap();
    flow.step().unwrap().unwrap();
    flow.remove_resource(removed).unwrap();
    flow.step().unwrap().unwrap();
    let reused = flow.create_resource(0).unwrap();
    assert_eq!(removed.entity_id().index, reused.entity_id().index);
    assert_ne!(
        removed.entity_id().generation,
        reused.entity_id().generation
    );
    let image = flow
        .capture_checkpoint_v4(&codecs(), FlowCheckpointLimits::default())
        .unwrap();
    assert!(!image.queue_order_modes.iter().any(|(id, _)| *id == removed));
    assert!(image.queue_order_modes.iter().any(|(id, _)| *id == reused));
    assert!(image.request_queue_orders.iter().any(
        |(id, order)| *id == request && matches!(order, FlowRequestQueueOrderV4::StableKey(_))
    ));
    assert!(image
        .request_queue_orders
        .iter()
        .any(|(id, order)| *id == second_request
            && matches!(order, FlowRequestQueueOrderV4::StableKey(_))));
    let decoded =
        kairo_ecs_des::FlowCheckpointV4::decode_wire_v4(&roundtrip(&image), wire_limits()).unwrap();
    assert_eq!(decoded.queue_order_modes, image.queue_order_modes);
    assert_eq!(decoded.request_queue_orders, image.request_queue_orders);
    let mut mismatch = image.clone();
    let (_, mismatched_order) = mismatch
        .request_queue_orders
        .iter_mut()
        .find(|(id, _)| *id == second_request)
        .unwrap();
    *mismatched_order = FlowRequestQueueOrderV4::AdmissionOrder;
    assert_eq!(
        mismatch.encode_wire_v4(wire_limits()),
        Err(FlowCheckpointWireError::InvalidValue(
            "queue-order metadata coverage"
        ))
    );
}

#[test]
fn v4_wire_literal_golden_and_legacy_bytes_unchanged() {
    // Independently derived from the empty FlowRuntime defaults and the frozen
    // V4 field inventory; this is not generated from a candidate encoder.
    const EMPTY_V4_GOLDEN_HEX: &str = concat!(
        "4b464c4f57563400010004000000a086010000000000000400000000000000000000000000000000",
        "00000000000000000100000000000000000000000000000000000000000000000000000000000000",
        "00000000000000000000000000000000000000000000000000000000000000000000000000000100",
        "00000000000000000000000000000000000000000000000000000000000000000000000000000000",
        "00000000000000000000000000000000000000000000000000000000000000000000000000000000",
        "00000000000000000000000000000000000000000000000000000000000000000000000000000000",
        "00000000000000000000000000000000000000000000000000000000000000000000000000000000",
        "00000000000000000000000000000000000000000000000000000000000000000000000000000000",
        "000000000000000000000000000000000000000000000000000000000000000000000000"
    );
    let flow = FlowRuntime::new();
    let registry = FlowCheckpointCodecs::new();
    let v1 = flow
        .capture_checkpoint(&registry, FlowCheckpointLimits::default())
        .unwrap();
    let v2 = flow
        .capture_checkpoint_v2(&registry, FlowCheckpointLimits::default())
        .unwrap();
    let v3 = flow
        .capture_checkpoint_v3(&registry, FlowCheckpointLimits::default())
        .unwrap();
    let limits = wire_limits();
    let old1 = v1.encode_wire_v1(limits).unwrap();
    let old2 = v2.encode_wire_v2(limits).unwrap();
    let old3 = v3.encode_wire_v3(limits).unwrap();
    assert_eq!(&old1[..8], b"KFLOWV1\0");
    assert_eq!(&old2[..8], b"KFLOWV2\0");
    assert_eq!(&old3[..8], b"KFLOWV3\0");
    assert_eq!(
        kairo_ecs_des::FlowCheckpointV1::decode_wire_v1(&old1, limits).unwrap(),
        v1
    );
    assert_eq!(
        kairo_ecs_des::FlowCheckpointV2::decode_wire_v2(&old2, limits).unwrap(),
        v2
    );
    assert_eq!(
        kairo_ecs_des::FlowCheckpointV3::decode_wire_v3(&old3, limits).unwrap(),
        v3
    );
    let image = flow
        .capture_checkpoint_v4(&registry, FlowCheckpointLimits::default())
        .unwrap();
    let bytes = image.encode_wire_v4(limits).unwrap();
    let (hex_pairs, remainder) = EMPTY_V4_GOLDEN_HEX.as_bytes().as_chunks::<2>();
    assert!(remainder.is_empty());
    let expected = hex_pairs
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(expected.len(), 356);
    assert_eq!(bytes, expected);
    assert_eq!(
        kairo_ecs_des::FlowCheckpointV4::decode_wire_v4(&expected, limits).unwrap(),
        image
    );
    assert_eq!(&bytes[..14], b"KFLOWV4\0\x01\0\x04\0\0\0");
    assert_eq!(&bytes[bytes.len() - 16..], &[0; 16]);
    let mut independently_framed = old3;
    independently_framed[..8].copy_from_slice(b"KFLOWV4\0");
    independently_framed[10..14].copy_from_slice(&FLOW_CHECKPOINT_VERSION_V4.to_le_bytes());
    independently_framed.extend_from_slice(&0_u64.to_le_bytes());
    independently_framed.extend_from_slice(&0_u64.to_le_bytes());
    assert_eq!(bytes, independently_framed);
}

#[test]
fn v4_wire_exact_byte_record_key_limits() {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let resource = flow
        .create_resource_with_queue_ordering(
            0,
            FlowAllocationMode::Eager,
            FlowQueueOrdering::StableKey,
        )
        .unwrap();
    let request = flow
        .acquire(resource)
        .owner(owner)
        .queue_order_key(FlowQueueOrderKey::new(&[b'k'; 128]).unwrap())
        .submit()
        .unwrap();
    flow.step().unwrap().unwrap();
    let image = flow
        .capture_checkpoint_v4(
            &FlowCheckpointCodecs::new(),
            FlowCheckpointLimits::default(),
        )
        .unwrap();
    assert!(image
        .request_queue_orders
        .iter()
        .any(|(id, order)| *id == request
            && matches!(order, FlowRequestQueueOrderV4::StableKey(k) if k.len() == 128)));
    let bytes = image.encode_wire_v4(wire_limits()).unwrap();
    let exact = FlowCheckpointWireLimits {
        max_wire_bytes: bytes.len(),
        ..wire_limits()
    };
    assert_eq!(image.encode_wire_v4(exact).unwrap(), bytes);
    assert_eq!(
        kairo_ecs_des::FlowCheckpointV4::decode_wire_v4(&bytes, exact).unwrap(),
        image
    );
    let exact_collections = FlowCheckpointWireLimits {
        flow: FlowCheckpointLimits {
            max_resources: image.resources.len(),
            max_requests: image.requests.len(),
            ..FlowCheckpointLimits::default()
        },
        ..wire_limits()
    };
    assert!(image.encode_wire_v4(exact_collections).is_ok());
    assert!(kairo_ecs_des::FlowCheckpointV4::decode_wire_v4(&bytes, exact_collections).is_ok());
    assert!(image
        .encode_wire_v4(FlowCheckpointWireLimits {
            flow: FlowCheckpointLimits {
                max_resources: image.resources.len() - 1,
                ..FlowCheckpointLimits::default()
            },
            ..wire_limits()
        })
        .is_err());
    assert!(image
        .encode_wire_v4(FlowCheckpointWireLimits {
            flow: FlowCheckpointLimits {
                max_requests: image.requests.len() - 1,
                ..FlowCheckpointLimits::default()
            },
            ..wire_limits()
        })
        .is_err());
    assert!(kairo_ecs_des::FlowCheckpointV4::decode_wire_v4(
        &bytes,
        FlowCheckpointWireLimits {
            flow: FlowCheckpointLimits {
                max_resources: image.resources.len() - 1,
                ..FlowCheckpointLimits::default()
            },
            ..wire_limits()
        }
    )
    .is_err());
    assert!(kairo_ecs_des::FlowCheckpointV4::decode_wire_v4(
        &bytes,
        FlowCheckpointWireLimits {
            flow: FlowCheckpointLimits {
                max_requests: image.requests.len() - 1,
                ..FlowCheckpointLimits::default()
            },
            ..wire_limits()
        }
    )
    .is_err());
    assert_eq!(
        image.encode_wire_v4(FlowCheckpointWireLimits {
            max_wire_bytes: bytes.len() - 1,
            ..wire_limits()
        }),
        Err(FlowCheckpointWireError::LimitExceeded("total byte"))
    );
    let exact_key = FlowCheckpointLimits {
        max_key_bytes: 128,
        ..FlowCheckpointLimits::default()
    };
    let exact_wire = FlowCheckpointWireLimits {
        flow: exact_key,
        ..wire_limits()
    };
    assert_eq!(image.encode_wire_v4(exact_wire).unwrap(), bytes);
    assert_eq!(
        kairo_ecs_des::FlowCheckpointV4::decode_wire_v4(&bytes, exact_wire).unwrap(),
        image
    );
    assert_eq!(
        image.encode_wire_v4(FlowCheckpointWireLimits {
            flow: FlowCheckpointLimits {
                max_key_bytes: 127,
                ..FlowCheckpointLimits::default()
            },
            ..wire_limits()
        }),
        Err(FlowCheckpointWireError::LimitExceeded("key bytes"))
    );
    assert_eq!(
        kairo_ecs_des::FlowCheckpointV4::decode_wire_v4(
            &bytes,
            FlowCheckpointWireLimits {
                flow: FlowCheckpointLimits {
                    max_key_bytes: 127,
                    ..FlowCheckpointLimits::default()
                },
                ..wire_limits()
            }
        ),
        Err(FlowCheckpointWireError::LimitExceeded("key bytes"))
    );
    let mut low = 0usize;
    let mut high = 4096usize;
    while low < high {
        let mid = low + (high - low) / 2;
        if kairo_ecs_des::FlowCheckpointV4::decode_wire_v4(
            &bytes,
            FlowCheckpointWireLimits {
                max_total_records: mid,
                ..wire_limits()
            },
        )
        .is_ok()
        {
            high = mid;
        } else {
            low = mid + 1;
        }
    }
    assert!(kairo_ecs_des::FlowCheckpointV4::decode_wire_v4(
        &bytes,
        FlowCheckpointWireLimits {
            max_total_records: low,
            ..wire_limits()
        }
    )
    .is_ok());
    assert!(kairo_ecs_des::FlowCheckpointV4::decode_wire_v4(
        &bytes,
        FlowCheckpointWireLimits {
            max_total_records: low - 1,
            ..wire_limits()
        }
    )
    .is_err());

    let (combined_image, _) = one_key_with_context_image();
    let string_key_bytes = dto_string_key_bytes(&combined_image);
    assert!(string_key_bytes > 0);
    let combined_key_cap = string_key_bytes + 128;
    assert!(combined_image
        .encode_wire_v4(FlowCheckpointWireLimits {
            flow: FlowCheckpointLimits {
                max_key_bytes: combined_key_cap,
                ..FlowCheckpointLimits::default()
            },
            ..wire_limits()
        })
        .is_ok());
    assert!(combined_image
        .encode_wire_v4(FlowCheckpointWireLimits {
            flow: FlowCheckpointLimits {
                max_key_bytes: combined_key_cap - 1,
                ..FlowCheckpointLimits::default()
            },
            ..wire_limits()
        })
        .is_err());
}

#[test]
fn v4_wire_checked_lengths_and_integer_overflow() {
    let image = FlowRuntime::new()
        .capture_checkpoint_v4(
            &FlowCheckpointCodecs::new(),
            FlowCheckpointLimits::default(),
        )
        .unwrap();
    let bytes = image.encode_wire_v4(wire_limits()).unwrap();
    assert!(kairo_ecs_des::FlowCheckpointV4::decode_wire_v4(
        &bytes,
        FlowCheckpointWireLimits {
            max_total_records: 0,
            ..wire_limits()
        }
    )
    .is_ok());
    assert_eq!(
        kairo_ecs_des::FlowCheckpointV4::decode_wire_v4(
            &bytes,
            FlowCheckpointWireLimits {
                max_wire_bytes: bytes.len() - 1,
                ..wire_limits()
            }
        ),
        Err(FlowCheckpointWireError::LimitExceeded("total byte"))
    );
    assert!(image
        .encode_wire_v4(FlowCheckpointWireLimits {
            max_total_records: 0,
            ..wire_limits()
        })
        .is_ok());
    let mut nonempty = FlowRuntime::new();
    let owner = nonempty.spawn_actor().unwrap();
    let resource = nonempty
        .create_resource_with_queue_ordering(
            0,
            FlowAllocationMode::Eager,
            FlowQueueOrdering::StableKey,
        )
        .unwrap();
    nonempty
        .acquire(resource)
        .owner(owner)
        .queue_order_key(FlowQueueOrderKey::new(b"record").unwrap())
        .submit()
        .unwrap();
    let nonempty = nonempty
        .capture_checkpoint_v4(
            &FlowCheckpointCodecs::new(),
            FlowCheckpointLimits::default(),
        )
        .unwrap();
    let nonempty_bytes = nonempty.encode_wire_v4(wire_limits()).unwrap();
    let zero_records = FlowCheckpointWireLimits {
        max_total_records: 0,
        ..wire_limits()
    };
    assert_eq!(
        nonempty.encode_wire_v4(zero_records),
        Err(FlowCheckpointWireError::LimitExceeded("aggregate record"))
    );
    assert_eq!(
        kairo_ecs_des::FlowCheckpointV4::decode_wire_v4(&nonempty_bytes, zero_records),
        Err(FlowCheckpointWireError::LimitExceeded("scheduler entry"))
    );
}

#[test]
fn v4_wire_header_tags_and_truncation_matrix() {
    let image = FlowRuntime::new()
        .capture_checkpoint_v4(
            &FlowCheckpointCodecs::new(),
            FlowCheckpointLimits::default(),
        )
        .unwrap();
    let valid = image.encode_wire_v4(wire_limits()).unwrap();
    let mut bad_magic = valid.clone();
    bad_magic[0] ^= 0xff;
    assert_eq!(
        kairo_ecs_des::FlowCheckpointV4::decode_wire_v4(&bad_magic, wire_limits()),
        Err(FlowCheckpointWireError::InvalidTag)
    );
    let mut bad_schema = valid.clone();
    bad_schema[8] = 2;
    assert_eq!(
        kairo_ecs_des::FlowCheckpointV4::decode_wire_v4(&bad_schema, wire_limits()),
        Err(FlowCheckpointWireError::UnsupportedSchema(2))
    );
    let mut bad_native = valid.clone();
    bad_native[10..14].copy_from_slice(&3_u32.to_le_bytes());
    assert_eq!(
        kairo_ecs_des::FlowCheckpointV4::decode_wire_v4(&bad_native, wire_limits()),
        Err(FlowCheckpointWireError::InvalidValue(
            "Flow checkpoint version"
        ))
    );
    for end in 0..valid.len() {
        assert!(
            kairo_ecs_des::FlowCheckpointV4::decode_wire_v4(&valid[..end], wire_limits()).is_err()
        );
    }
    assert_eq!(
        kairo_ecs_des::FlowCheckpointV4::decode_wire_v4(
            &[valid.as_slice(), &[0]].concat(),
            wire_limits()
        ),
        Err(FlowCheckpointWireError::TrailingBytes)
    );
    let mut bad_option = valid.clone();
    assert_eq!(bad_option[38], 0);
    bad_option[38] = 2;
    assert_eq!(
        kairo_ecs_des::FlowCheckpointV4::decode_wire_v4(&bad_option, wire_limits()),
        Err(FlowCheckpointWireError::InvalidTag)
    );

    let mut mode_tag = one_stable_key_wire(b"unique-mode-marker");
    let mode_key_at = mode_tag
        .windows(b"unique-mode-marker".len())
        .rposition(|window| window == b"unique-mode-marker")
        .unwrap();
    let mode_tag_at = mode_key_at - 30;
    assert_eq!(mode_tag[mode_tag_at], 1);
    mode_tag[mode_tag_at] = 2;
    assert_eq!(
        kairo_ecs_des::FlowCheckpointV4::decode_wire_v4(&mode_tag, wire_limits()),
        Err(FlowCheckpointWireError::InvalidTag)
    );
    let mut unsorted_modes = two_resource_modes_wire();
    let mode_row_len = 13;
    let mode_rows_start = unsorted_modes.len() - 8 - 2 * mode_row_len;
    let first_mode = unsorted_modes[mode_rows_start..mode_rows_start + mode_row_len].to_vec();
    let second_mode =
        unsorted_modes[mode_rows_start + mode_row_len..mode_rows_start + 2 * mode_row_len].to_vec();
    assert_eq!(first_mode.len(), mode_row_len);
    assert_eq!(second_mode.len(), mode_row_len);
    unsorted_modes[mode_rows_start..mode_rows_start + mode_row_len].copy_from_slice(&second_mode);
    unsorted_modes[mode_rows_start + mode_row_len..mode_rows_start + 2 * mode_row_len]
        .copy_from_slice(&first_mode);
    assert_eq!(
        kairo_ecs_des::FlowCheckpointV4::decode_wire_v4(&unsorted_modes, wire_limits()),
        Err(FlowCheckpointWireError::InvalidValue(
            "queue-order metadata canonical order"
        ))
    );
    let mut order_tag = one_stable_key_wire(b"unique-key");
    let key_at = order_tag
        .windows(b"unique-key".len())
        .rposition(|window| window == b"unique-key")
        .unwrap();
    order_tag[key_at - 9] = 2;
    assert_eq!(
        kairo_ecs_des::FlowCheckpointV4::decode_wire_v4(&order_tag, wire_limits()),
        Err(FlowCheckpointWireError::InvalidTag)
    );
    let mut empty_key = one_stable_key_wire(b"unique-key");
    let key_at = empty_key
        .windows(b"unique-key".len())
        .rposition(|window| window == b"unique-key")
        .unwrap();
    empty_key[key_at - 8..key_at].copy_from_slice(&0_u64.to_le_bytes());
    assert_eq!(
        kairo_ecs_des::FlowCheckpointV4::decode_wire_v4(&empty_key, wire_limits()),
        Err(FlowCheckpointWireError::InvalidValue(
            "stable queue-order key"
        ))
    );
    let mut oversize_key = one_stable_key_wire(b"unique-key");
    let key_at = oversize_key
        .windows(b"unique-key".len())
        .rposition(|window| window == b"unique-key")
        .unwrap();
    oversize_key[key_at - 8..key_at].copy_from_slice(&129_u64.to_le_bytes());
    assert_eq!(
        kairo_ecs_des::FlowCheckpointV4::decode_wire_v4(&oversize_key, wire_limits()),
        Err(FlowCheckpointWireError::LimitExceeded(
            "queue order key bytes"
        ))
    );
}

#[test]
fn v4_wire_duplicate_missing_unsorted_metadata_rejected() {
    let (flow, registry, _, _, _, _, _, _, _) = mixed_seed();
    let image = flow
        .capture_checkpoint_v4(&registry, FlowCheckpointLimits::default())
        .unwrap();
    assert_eq!(image.version, FLOW_CHECKPOINT_VERSION_V4);
    assert_eq!(
        image.encode_wire_v4(wire_limits()).unwrap(),
        roundtrip(&image)
    );
    let mut missing = image.clone();
    missing.request_queue_orders.pop();
    assert_eq!(
        missing.encode_wire_v4(wire_limits()),
        Err(FlowCheckpointWireError::InvalidValue(
            "queue-order metadata coverage"
        ))
    );
    let mut missing_mode = image.clone();
    missing_mode.queue_order_modes.pop();
    assert_eq!(
        missing_mode.encode_wire_v4(wire_limits()),
        Err(FlowCheckpointWireError::InvalidValue(
            "queue-order metadata coverage"
        ))
    );
    let mut duplicate_mode = image.clone();
    duplicate_mode.queue_order_modes[1].0 = duplicate_mode.queue_order_modes[0].0;
    assert_eq!(
        duplicate_mode.encode_wire_v4(wire_limits()),
        Err(FlowCheckpointWireError::InvalidValue(
            "queue-order metadata canonical order"
        ))
    );
    let mut unknown_mode = image.clone();
    let mut foreign = FlowRuntime::new();
    let foreign_resource = loop {
        let candidate = foreign.create_resource(0).unwrap();
        if candidate.entity_id().index > image.world.slots.len() as u64 {
            break candidate;
        }
    };
    assert!(!image.resources.contains(&foreign_resource));
    assert!(image
        .resources
        .iter()
        .all(|resource| resource.entity_id().index < foreign_resource.entity_id().index));
    unknown_mode.queue_order_modes[1].0 = foreign_resource;
    assert_eq!(
        unknown_mode.encode_wire_v4(wire_limits()),
        Err(FlowCheckpointWireError::InvalidValue(
            "queue-order metadata coverage"
        ))
    );
    let mut mismatch = image.clone();
    mismatch.request_queue_orders[0].1 = FlowRequestQueueOrderV4::AdmissionOrder;
    assert_eq!(
        mismatch.encode_wire_v4(wire_limits()),
        Err(FlowCheckpointWireError::InvalidValue(
            "queue-order metadata coverage"
        ))
    );
    let mut duplicate_request = image.clone();
    let first_request_id = duplicate_request.request_queue_orders[0].0;
    let last = duplicate_request.request_queue_orders.len() - 1;
    duplicate_request.request_queue_orders[last].0 = first_request_id;
    assert_eq!(
        duplicate_request.encode_wire_v4(wire_limits()),
        Err(FlowCheckpointWireError::InvalidValue(
            "queue-order metadata canonical order"
        ))
    );
    let mut unknown_request = image.clone();
    let last = unknown_request.request_queue_orders.len() - 1;
    let foreign_request = foreign_request_id();
    assert!(!image.requests.contains(&foreign_request));
    unknown_request.request_queue_orders[last].0 = foreign_request;
    assert_eq!(
        unknown_request.encode_wire_v4(wire_limits()),
        Err(FlowCheckpointWireError::InvalidValue(
            "queue-order metadata coverage"
        ))
    );
    let duplicate_key = image.clone();
    let stable = duplicate_key
        .request_queue_orders
        .iter()
        .filter_map(|(_, row)| match row {
            FlowRequestQueueOrderV4::StableKey(bytes) => Some(bytes.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(stable.windows(2).any(|pair| pair[0] == pair[1]));
    assert!(duplicate_key.encode_wire_v4(wire_limits()).is_ok());
    let mut unsorted = two_equal_length_request_keys_wire(b"same");
    let row_len = 12 + 1 + 8 + 4;
    let rows_start = unsorted.len() - 2 * row_len;
    let request_count_end = unsorted.len();
    let first = unsorted[rows_start..rows_start + row_len].to_vec();
    let second = unsorted[rows_start + row_len..request_count_end].to_vec();
    unsorted[rows_start..rows_start + row_len].copy_from_slice(&second);
    unsorted[rows_start + row_len..request_count_end].copy_from_slice(&first);
    assert_eq!(
        kairo_ecs_des::FlowCheckpointV4::decode_wire_v4(&unsorted, wire_limits()),
        Err(FlowCheckpointWireError::InvalidValue(
            "queue-order metadata canonical order"
        ))
    );
}

#[test]
fn v4_wire_bad_final_row_and_trailing_precede_owned_decode() {
    let mut bytes = FlowRuntime::new()
        .capture_checkpoint_v4(
            &FlowCheckpointCodecs::new(),
            FlowCheckpointLimits::default(),
        )
        .unwrap()
        .encode_wire_v4(wire_limits())
        .unwrap();
    bytes.push(0);
    assert_eq!(
        kairo_ecs_des::FlowCheckpointV4::decode_wire_v4(&bytes, wire_limits()),
        Err(FlowCheckpointWireError::TrailingBytes)
    );
}

#[test]
fn v4_wire_measurement_precedes_output_allocation() {
    let image = FlowRuntime::new()
        .capture_checkpoint_v4(
            &FlowCheckpointCodecs::new(),
            FlowCheckpointLimits::default(),
        )
        .unwrap();
    assert!(matches!(
        image.encode_wire_v4(FlowCheckpointWireLimits {
            max_wire_bytes: 1,
            ..wire_limits()
        }),
        Err(FlowCheckpointWireError::LimitExceeded("total byte"))
    ));
    assert!(image.encode_wire_v4(wire_limits()).is_ok());
}

#[test]
fn v4_wire_direct_restore_has_no_legacy_fallback() {
    let (flow, registry, _, _, _, _, _, _, _) = mixed_seed();
    let image = flow
        .capture_checkpoint_v4(&registry, FlowCheckpointLimits::default())
        .unwrap();
    let bytes = image.encode_wire_v4(wire_limits()).unwrap();
    assert_eq!(
        kairo_ecs_des::FlowCheckpointV4::decode_wire_v4(&bytes, wire_limits()).unwrap(),
        image
    );
    assert!(matches!(
        kairo_ecs_des::FlowCheckpointV3::decode_wire_v3(&bytes, wire_limits()),
        Err(FlowCheckpointWireError::InvalidTag)
    ));
    let resource = image.resources[0];
    assert_eq!(
        FlowRuntime::restore_checkpoint_v4(image, &registry, FlowCheckpointLimits::default())
            .unwrap()
            .resource_queue_ordering(resource)
            .unwrap(),
        FlowQueueOrdering::StableKey
    );
}
