# Track 53: FMI 2/3 Co-Simulation Master Runtime

## Purpose

Complete FMI interoperability by adding `.fmu` archive handling, dynamic
library loading, FMI 2/3 lifecycle calls, KairoECS co-simulation master
coordination, OpenModelica round trips, and 1,000-step runtime proof.

## Maturity

Spec Approved planning track. The current implementation remains the Track 38
unpacked-layout scaffold until live FMU runtime proof closes.

## Inputs

- `crates/kairo-ecs-fmi/` and examples from Track 38.
- FFI safety contracts from Track 02.
- Interoperability mapping from Track 26.
- Evidence manifest from Track 46.

## Outputs

- FMU zip extraction and validation.
- Dynamic symbol binding through safe lifecycle wrappers.
- FMI 2 and FMI 3 co-simulation master stepping.
- Exported FMU package validation.
- OpenModelica or equivalent third-party import/export roundtrip evidence.

## Owned paths

- `crates/kairo-ecs-fmi/`
- `examples/fmi-co-simulation/`
- `docs/fmi-digital-twin/`
- `conductor/tracks/53-fmi-cosimulation-master-runtime/`

## Blocked paths

- Generic FFI ABI changes without Track 02 handoff.
- Streaming broker integration owned by Track 36.
- End-to-end scaling certification owned by Track 55.

## Dependencies

Tracks 38, 46, and 02.

## Parallel-safe tracks

Track 55 may draft integrated FMI scenarios after this track defines the FMU
runtime command contract.

## Acceptance criteria

- `.fmu` archives are unpacked, validated, and rejected safely on malformed
  input.
- Dynamic library symbols are loaded through explicit FMI lifecycle wrappers.
- FMI 2 and FMI 3 co-simulation steps run for at least 1,000 steps.
- Exported FMUs pass third-party tool import checks.
- All unsafe boundaries are audited and documented.

## Quality gates

- `fmu-archive-roundtrip`
- `fmi-dynamic-loading`
- `fmi2-cosim-1000-step`
- `fmi3-cosim-1000-step`
- `openmodelica-roundtrip`
- `phase-closeout-check`

## Release implications

This track gates production FMI, FMU, and co-simulation master claims.
