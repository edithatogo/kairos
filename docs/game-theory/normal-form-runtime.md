# Normal-Form Runtime

Track 60 adds a dependency-light normal-form execution surface to `kairo-ecs-game-theory`.
The current runtime is a flat-array solver contract, not a release performance claim.

## Data layout

- `StrategySpace` stores player-ordered strategy names.
- `PayoffMatrix` stores payoffs contiguously by profile, then player.
- `Utility` rejects non-finite values at construction.
- Invalid profiles and players are rejected with typed errors or `None` lookups.

For a two-player, two-strategy game, payoff storage follows this order:

1. profile `[0, 0]`, player `0`, player `1`
2. profile `[0, 1]`, player `0`, player `1`
3. profile `[1, 0]`, player `0`, player `1`
4. profile `[1, 1]`, player `0`, player `1`

## Solver surface

- `BestResponseSolver::best_responses(matrix, player, opponent_profile)` returns all utility-maximizing target-player strategies and preserves tie order.
- `PureNashSolver::equilibria(matrix)` enumerates pure Nash equilibria in deterministic profile order.
- `StrictDominanceSolver::strictly_dominated_strategies(matrix, player)` reports strategies that are strictly dominated by another strategy for every opponent profile.

## Local gates

```text
rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-game-theory --test normal_form
rustup run stable-x86_64-pc-windows-gnu cargo run -p kairo-ecs-game-theory --example normal_form_runtime
rustup run stable-x86_64-pc-windows-gnu cargo bench -p kairo-ecs-game-theory --bench normal_form -- --quick
```

## Boundaries

The Track 60 runtime does not yet provide mixed-strategy solvers, generated-ontology integration, extensive-form traversal, or externally benchmarked parity. Extensive-form Graph-ECS execution remains Track 61 scope.
