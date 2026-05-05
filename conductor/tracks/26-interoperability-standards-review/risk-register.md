# Risk Register: Track 26 Interoperability Standards Review

| Risk | L | I | Sev | Mitigation | Owner | Escalation trigger |
|---|---|---|---|---|---|---|
| Interoperability standard chosen without binding conformance requirement | 3 | 4 | 12 | Every adopted standard must map to a CI-checked conformance assertion | interop-agent | Standard referenced in spec without CI conformance check |
| Cross-binding data format drift | 3 | 4 | 12 | Contract-first workflow; shared Arrow schema and C header as single source of truth | contracts-agent | Any binding diverges from canonical schema |
| Standard version compatibility not tracked | 3 | 4 | 12 | Maintain a standards-version matrix; test against minimum and current versions in CI | interop-agent | Version matrix stale or missing for any adopted standard |
| Interoperability claim exceeds implemented capability | 3 | 4 | 12 | Mark maturity status for each standard; require conformance fixtures | docs-agent | Interop claim published without conformance fixture pass |
| Standards body changes break compliance | 3 | 3 | 9 | Monitor standards-body release feeds; scheduled CI against latest spec versions | interop-agent | Standards-body update triggers unplanned compliance work |
