# Conformance Contract

## Fixture format

Fixtures live under:

```text
conformance/fixtures/<fixture-name>/
```

Each fixture includes:

```text
input.json
expected_trace.json
expected_summary.json
expected_arrow_schema.json
README.md
```

## Required v1 fixtures

```text
scheduler_ordering_v1
scheduler_cancellation_v1
zero_delay_guard_v1
rng_reproducibility_v1
des_resource_queue_v1
abm_behavior_update_v1
hybrid_des_abm_v1
arrow_event_log_v1
ffi_lifecycle_v1
```

## Binding requirement

Every host binding must expose a test runner that can run the shared fixtures and produce a conformance report.

## Pass criteria

```text
same final simulation time
same dispatched event IDs/kinds/order
same cancellation behavior
same summary metrics
same Arrow schema fingerprint
no leaks/double-free/panic across FFI
```
