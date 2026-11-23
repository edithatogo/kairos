# Debugger CLI

`kairo-ecs-debug replay` runs a seeded Rust model and reads one command per
line from standard input. The model dispatches two events per tick, schedules
work during dispatch, and updates a running total with the repository's
deterministic RNG. End of input closes the session. An invalid command or an
unavailable position ends it with an error.

```bash
printf 'step\ninspect total\nstep\nback\ninspect total\nbreak 2\ncontinue\n' |
  cargo run --locked -p kairo-ecs-debug -- replay --events 8 --seed 7
```

| Command | Behaviour |
| --- | --- |
| `step` | Dispatch the next event, apply its transition and print its cursor. |
| `back` | Restore the state immediately before the current event. |
| `seek <position>` | Restore an observed event position. Position zero is the initial state. |
| `goto <tick>` | Restore the first retained event dispatched at or after this tick. |
| `inspect <applied\|total\|rng-words\|pending>` | Read model state, RNG position or pending scheduler count at the cursor. |
| `break <kind>` | Add a custom event-kind breakpoint. |
| `list-breakpoints` | Print the active event kinds. |
| `continue` | Dispatch until a breakpoint fires or the queue is empty. |

The cursor identifies a specific event when several events share a tick.
Change the retention with `--checkpoint-every` and `--checkpoints`.
Seek and back return errors outside the retained history. `continue` ends
at a matching breakpoint after its transition runs.

`kairo-ecs-debug record --events 8` prints the existing scheduler statistics
example. Rust applications can use [`ReplaySession`][adr] to supply their
own deterministic model and fork independent timelines in process.

The hosted debug crate tests exercise the executable and Rust library.
The [browser timeline demo](../../website/time-travel-demo/index.html)
plays a checked-in trace fixture.

[adr]: ../adr/0020-native-debugger-replay.md
