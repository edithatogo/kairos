# Task 0.3 Qualification Receipt: Polars / Arrow Version, License, MSRV & Isolation

## 1. Candidate Dependency Pin

| Crate | Selected Version Pin | License | Direct Dependencies | Purpose |
| :--- | :--- | :--- | :--- | :--- |
| `polars` | `0.55.2` | MIT | `polars-core`, `polars-ops`, `polars-lazy` | High-performance tabular transformation and query engine. |
| `polars-arrow` | `0.55.2` | MIT / Apache-2.0 | `arrow2` lineage | In-memory Arrow format utilized by Polars. |
| `arrow-schema` | `60.0.0` | Apache-2.0 | None (minimal) | Logical type contracts and Arrow schema definitions. |

### Feature Set Selection
For `kairo-ecs-arrow`:
- `polars-native`: optional feature admitting `polars = { version = "0.55.2", default-features = false, features = ["lazy", "parquet", "ipc", "fmt"] }`.
- Default features: `[]` (dependency-light smoke and core schema tests compile with zero native C/heavy dependencies).

---

## 2. License & Security Advisory Audit

- **License Compatibility**:
  - `polars` is licensed under `MIT`.
  - `polars-arrow` is licensed under `MIT` / `Apache-2.0`.
  - `arrow-schema` is licensed under `Apache-2.0`.
  - All licenses are compatible with the Kairos workspace license (`MIT OR Apache-2.0`).
  - No GPL/copyleft or problematic dependencies introduced.
- **Vulnerability / Security Advisories**:
  - Audited via `cargo audit` / GitHub Security Advisory Database. No active unpatched advisories on `polars 0.55.2`.

---

## 3. MSRV & Toolchain Constraints

- **Workspace MSRV**: `1.99` (declared in `Cargo.toml`).
- **Active Toolchain**: `rustc 1.99.1` (Homebrew).
- **Polars MSRV**: Compatible with Rust 1.82+; builds cleanly on Rust 1.99.

---

## 4. Feature Isolation Proof

1. **Minimal Core (`kairo-ecs-core`, `kairo-ecs-types`)**:
   - `cargo tree -p kairo-ecs-core` confirms zero dependencies on `polars`, `arrow`, or `parquet`. Core simulation identity, scheduler, and RNG remain completely decoupled from tabular analytics.
2. **WebAssembly / Browser (`kairo-ecs-wasm`)**:
   - `cargo tree -p kairo-ecs-wasm` confirms that `kairo-ecs-wasm` depends only on `kairo-ecs-ffi`, `kairo-ecs-core`, `kairo-ecs-state`, `kairo-ecs-rng`, and `kairo-ecs-types`.
   - `polars` is excluded from the WebAssembly dependency graph.
3. **Table Representation**:
   - There is no duplicate full table representation. In Phase 1, `polars::prelude::DataFrame` will serve as the canonical tabular engine in `kairo-ecs-arrow`, replacing the hand-rolled `ArrowColumnBuffer`.
