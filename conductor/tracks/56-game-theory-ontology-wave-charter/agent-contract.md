# Track 56 Agent Contract

## Agents

- `ontology-agent` owns ontology evidence shape.
- `game-theory-agent` owns solver and game semantics claim boundaries.
- `wave-manager-agent` owns dependency ordering and lifecycle enforcement.

## Rules

- Commit after every task.
- Do not advance any downstream track without its own task commits, phase review/push record, and GitHub Actions review.
- Do not mark live capability as complete from documentation or scaffolds alone.
- Record every waiver with owner, expiry condition, and replacement evidence.
