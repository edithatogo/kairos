# Debugger CLI Reference

Planned commands:

- `step`: move the cursor to the next event delta.
- `back`: move the cursor to the previous event delta.
- `goto <tick>`: move to the first event at or after a tick.
- `inspect <entity-or-key>`: print reconstructed state at the cursor.
- `break on <event-kind>`: add an event-kind breakpoint.
- `list-breakpoints`: show active breakpoints.

The first scaffold implements the command names and the library behavior behind stepping, backward movement, tick seek, inspection, and breakpoint matching.
