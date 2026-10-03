# Experimental Q2 priority and waiting deadline contract

Independent source review 2026-10-03 against frozen Q0 traces. One coordinator
owns flow.rs. Extend AcquireBuilder priority(i32), deadline(SimTime), and
scheduler_priority(i32). Add cancel/request and reprioritize/request commands
with explicit priority override variants; defaults remain zero. Smaller queue
priority wins; original committed admission sequence survives rekey.

Requests store submitted_at and optional waiting deadline. Grant clears deadline.
TimedOut is terminal. Waiting deadline <= dispatch time expires before any grant,
even if the timeout event was inserted after release/growth/rekey. Explicit
stale/rejected command retains independently committed timeout rows. Preflight
arithmetic failure preserves the whole staged transaction. Terminal resubmission
uses a new request. Deadline <= admission time is immediate timeout; no schedule
in the past is generated. Both admission and timeout event budgets are checked
before allocation. Deadline tokens after grant/cancel are harmless stale no-ops.
Cancellation may terminate pending, queued or active requests and releases capacity.
Repriority updates pending/queued/active authoritative priority and allocation;
queued index is rebuilt with original sequence. Scheduler order remains core order.
Timed completion-before-eviction remains Q3. No handlers or portable codecs.
