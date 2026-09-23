# ADR 0017: v4 resource release decision

Status: Accepted by the operator on 2026-09-23; expires after 4.0.0.

## Context and authorization

Completed native resource windows show small original-budget overruns on macOS
and Linux. All measured candidate sessions shut down cleanly. Windows passes
the original resource limits, including its unobserved two-hour soak. Exact
measurements and original failed verdicts remain under `docs/qualification/v4`.

After reviewing the measured overruns, the operator answered "Yeah go for it"
to the explicit proposal of 4 percent foreground CPU, 3 percent hidden CPU,
and 200 MiB peak memory for v4 only. The operator subsequently required all
original goals to be written in the repository as next-version targets.

## Decision

For **4.0.0 only**, all six platforms may use at most 4 percent of one logical
core in the foreground (CLI/TUI/GUI), 3 percent while the GUI is hidden, and
200 MiB peak summed process-family RSS/working set. The 300 MiB private-memory
limit remains unchanged. ADR 0016 separately permits frame/input p95 and
ordinary-refresh maximum of at most 100 ms for this release.

Assess the retained measurements separately from the original verdicts; never
rewrite a failed original-target result. The qualification policy automatically
uses original limits for every other product version. Missing/nonfinite data,
short windows, diagnostic/observer runs, failed shutdown, terminal restoration
failure and functional failures cannot pass through this exception. Collection
cadence, native accessibility/input fixtures, security, lifecycle, the two-hour
soak and public artifact verification remain mandatory.

This supersedes only ADR 0016's statement that CPU/RSS release limits remain
unchanged. It does not change product code or measurement methodology.

## Follow-up

Codex owns #r17 for CPU/memory improvements and #r16 for responsiveness.
[Next-version targets](../next-version-targets.md) preserve the complete
original targets, measurement rules and functional goals. A new waiver is not
implied for a later patch or minor version.
