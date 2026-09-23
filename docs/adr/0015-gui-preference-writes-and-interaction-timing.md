# ADR 0015 — Keep preference writes outside GUI interaction

Date: 2026-09-23
Status: Accepted
Related: `.tasks/tasks/v4a.md`, `gui/src/settings_writer.zig`,
`scripts/qualify-gui-interaction.py`

## Context and evidence

Navigation persists the selected section. The original synchronous path wrote,
flushed and atomically replaced preferences before finishing the input event.
Complete-event timing on Windows exceeded the 16.7 ms frame p95 budget during
navigation; an unchanged repeat confirmed it. An independent forty-write probe
measured 5.51 ms median and 11.26 ms p95 for the atomic settings transaction.
Raster-only measurements had not included this cost.

## Decision

Each GUI process owns one preference writer and one replaceable pending document.
The writer performs the existing namespace-preserving atomic transaction outside
the queue lock. Rapid changes replace pending preferences; a write already in
progress completes before the newest pending document is written. The model
tracks request and completion sequences and shows pending or failed persistence.
Shutdown flushes the latest pending request and joins the writer before unloading
the engine, including the explicit AppKit termination path.

Do not spawn a thread for each click or silently abandon pending writes on exit.
The TUI continues to own its independent state and settings namespace.

Interaction qualification measures synchronous runtime work across input,
update, layout, raster and presentation dispatch. Nested events count once;
queue waits do not count as work. A cycle that produces no repaint ends when
its synchronous work completes rather than accumulating unrelated idle ticks.
Input latency runs from runtime receipt to the responding present. It does not
measure physical input hardware or display scanout.

Automation is compiled into a separate qualification stage. The harness checks
the publishing process identity, bounds snapshot reads and command deadlines,
requires complete percentile coverage, and owns process cleanup. Resource and
soak measurements use the ordinary build without an automation observer.

## Consequences and qualification

Fixtures cover pending-write coalescing, slow writes without queue-lock
contention, failure/recovery, joined shutdown, nested timing, idle gaps and
bounded percentile windows. Revision 0069088 passes Windows interaction gates
at compact and default viewports in both audience modes: worst cohort work p95
14.503 ms, input p95 40.591 ms, and ordinary refresh maximum 9.816 ms. The retained
report records the exact GUI, engine and collector hashes.

The coordinated 4.0.0 candidate adds no-damage cycle closure and native harness
execution on all six targets. Those native results and resource/soak acceptance
remain independent release gates; this decision does not claim they have passed.
