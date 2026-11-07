# Extensive-Form Graph-ECS Runtime & Solvers

This page covers graph representation and backward induction for supported
perfect-information games. Information-set storage alone does not establish
general imperfect-information solving; see [CFR scope and S04 qualification](equilibrium-and-cfr-solvers.md).

## Overview

Track 61 implements extensive-form game representations and backward induction solvers over KairoECS's flat component stores. Games are represented as directed trees or DAGs composed of decision nodes, chance nodes, action edges, information sets, and terminal utility allocations.

## Node & Edge Taxonomy

Under the `graph-relations` feature, `kairo_ecs_game_theory::extensive_form` provides:

- **`ExtensiveNode`**: Classifies tree vertices into:
  - `Decision { player: usize }`: A player chooses among available actions.
  - `Chance`: Nature chooses among outcomes according to fixed probabilities.
  - `Terminal`: Leaf node yielding payoffs to all participating players.
- **`ActionEdge`**: Represents a player action leading to a child node with a descriptive label.
- **`ChanceOutcome`**: Represents a stochastic branch with a label, positive probability, and target node.
- **`TerminalUtility`**: Holds payoff values for players at terminal nodes.
- **`InformationSet`**: Groups multiple decision nodes together for games with imperfect information.

## Solvers

### 1. Backward Induction Solver

`BackwardInductionSolver` solves finite perfect-information sequential games by recursively finding optimal actions from the terminal leaves back to the root:

```rust
use kairo_ecs_game_theory::extensive_form::BackwardInductionSolver;

let solution = BackwardInductionSolver::solve(root, &nodes, &terminals, stores)?;
println!("Best root action: {:?}", solution.best_action);
println!("Expected payoff: {:?}", solution.expected_payoff);
```

### 2. Chance-Weighted Backward Induction Solver

`ChanceWeightedBackwardInductionSolver` handles extensive-form games containing chance/stochastic nodes (such as card deals, dice rolls, or uncertain nature states):

- Aggregates expected utilities across all chance branches weighted by outcome probabilities.
- Maximizes expected payoffs at decision nodes based on downstream chance expectations.

## Multi-Game Certification

The following validator checks scenario metadata and source constraints. Its
self-test does not execute the listed games or prove generalized solver behavior;
independent native execution and mathematical validation remain separate gates.

- `node scripts/validation/validate-multigame-certification.mjs --self-test`
- Manifest: `conductor/game-theory-evidence/templates/track61-multigame-certification.json`
