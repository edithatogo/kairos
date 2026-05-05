# Risk Register: Track 26 Interoperability Standards Review

| Risk | L | I | Sev | Mitigation | Owner | Escalation trigger |
|---|---|---|---|---|---|---|
| Interoperability standard chosen without binding conformance requirement | 3 | 4 | 12 | Every adopted standard must map to a CI-checked conformance assertion | interop-agent | Standard referenced in spec without CI conformance check |
| Cross-binding data format drift | 3 | 4 | 12 | Contract-first workflow; shared Arrow schema and C header as single source of truth | contracts-agent | Any binding diverges from canonical schema |
| Standard version compatibility not tracked | 3 | 4 | 12 | Maintain a standards-version matrix; test against minimum and current versions in CI | interop-agent | Version matrix stale or missing for any adopted standard |
| Interoperability claim exceeds implemented capability | 3 | 4 | 12 | Mark maturity status for each standard; require conformance fixtures | docs-agent | Interop claim published without conformance fixture pass |
| Standards body changes break compliance | 3 | 3 | 9 | Monitor standards-body release feeds; scheduled CI against latest spec versions | interop-agent | Standards-body update triggers unplanned compliance work |
| Arrow IPC or Parquet support overstated from smoke-byte evidence | 3 | 4 | 12 | Label Arrow IPC and Parquet as deferred until real serializers/readers and conformance fixtures exist; validator checks both standards are present in release guards | interop-agent | Release notes or docs claim Arrow IPC/Parquet support without fixture evidence |
| FMI/FMU scaffold mistaken for arbitrary FMU execution support | 3 | 4 | 12 | Label FMI/FMU as partial and name exact supported surfaces: unpacked-layout checks, lifecycle wrapper, and unpacked export layout generation | fmi-agent | Docs claim third-party FMU execution before dynamic loading and OpenModelica round-trip evidence |
| OpenTelemetry naming guidance mistaken for OTLP export support | 3 | 3 | 9 | Label OpenTelemetry semantic conventions as partial and require exporter/fixture evidence before native OTel language appears | interop-agent | Docs mention OTel export, OTLP, or collector compatibility without implementation |
| Unsupported ecosystem comparisons become compatibility claims | 2 | 4 | 8 | Keep Mesa, Agents.jl, MASON, NetLogo, SimPy, simmer, ConcurrentSim.jl, SimSharp, and AnyLogic-style mappings under unsupported comparison references | docs-agent | Migration docs imply behavioral equivalence or import/export support |
