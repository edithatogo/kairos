# Interoperability Standards Review Plan

## Standards and ecosystems to evaluate

| Area | Why it matters | Initial stance |
|---|---|---|
| DEVS | DES formalism and hierarchical modeling | Map concepts, do not implement initially |
| FMI/FMU | Digital twins and co-simulation | Research before Track 1.0+ |
| SBML/CellML | Systems biology and continuous models | Out of scope for DES/ABM MVP but document bridge |
| OpenTelemetry | Traces/logs/metrics vocabulary | Use concepts where useful; avoid forcing telemetry output into OTel |
| Apache Arrow schemas | Cross-language analytics | First-class |
| SimPy/Mesa/Agents.jl/simmer mappings | Migration and adoption | Provide migration guides |

## Output

- `docs/interoperability/*.md`
- migration tables
- feature comparison tables
- future roadmap ADRs
