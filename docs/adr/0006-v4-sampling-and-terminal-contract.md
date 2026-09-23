# ADR 0006 — Independent bounded sampling and the v4 terminal contract

Status: Accepted for the v4 implementation (operator plan, 2026-09-23).

## Context

The operator explicitly authorized a terminal redesign and correctness changes.
The old loop performed discovery before its first draw, ran several slow probes
on the input loop, and appended old GPU/thermal readings on every fast tick.
Native calls on detached Rust threads also made shutdown depend on the provider.

## Decision

Both frontends instantiate the shared `Monitor` implementation independently.
Each provider lane owns one worker, one replaceable result, and its next deadline.
Sampling retains the established fast/connection/slow/diagnostic/health cadences.
Overdue ticks are skipped. The GUI retains its lightweight page/hidden profiles.
Discovery is cached between inventory refreshes; explicit retry wakes a lane.
Failures back off and retain the previous capture timestamp with an error state.

Slow/native probes run through the same installed CLI's hidden, enumerated
`collect-worker` protocol. A versioned typed response is required. The parent
owns the process tree, deadline, cancellation flag, and bounded file capture.
It cancels and joins workers before terminal exit or engine-library unload.
CPU, memory, and counter sampling remain in-process; helper-backed discovery,
SMART, thermal bridges, drivers, sockets, and reachability cannot block input.
The worker never accepts an arbitrary command, changes hardware, or installs tools.

Capture time and sequence belong to collection, not rendering. Missing readings
are gaps. A history never combines CPU and GPU temperature or advances merely
because a screen redraws. Topic envelopes carry the actual sample's metadata;
old values remain attributable when a provider fails and recovers.

The authorized TUI changes are additive filtering, inspection, page navigation,
stable row selection, optional mouse, pause-view, adaptive layout, explicit
freshness, and restrained motion with terminal fallbacks. Existing mode chooser,
nine section shortcuts, lifecycle commands, and restoration remain supported.
The new `tui` preferences stay separate from `gui`; provider choices deliberately
shared by both frontends belong under `shared`.

## Consequences and qualification

The GUI now requires its matching CLI for isolated probes, already supplied by
the composite product. Missing or mismatched companions produce explicit provider
errors while fast monitoring remains available. Child-process startup costs must
be included in the performance qualification, not hidden in renderer timings.
Each incremental feature commit is qualified locally and on the six hosted native
targets. Only the complete candidate may enter the single public v4 release.
