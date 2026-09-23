# ADR 0016: v4 responsiveness release decision

Status: Accepted by the operator on 2026-09-23; expires after 4.0.0.

## Context

Native interaction qualification on Windows, both GNU Linux architectures,
Linux musl, and Apple Silicon completed with frame-work p95 between 17.926 and
27.979 ms, input p95 between 25.484 and 45.218 ms, and ordinary-refresh maxima
between 5.750 and 20.182 ms. These are measurements of event processing through
host presentation, not physical display scanout. Intel Mac qualification was
still running when this decision was made.

The original plan required frame p95 at most 16.7 ms, input p95 at most 50 ms,
and no ordinary refresh over 100 ms. On 2026-09-23 the operator approved 100 ms
responsiveness for this release, first for Windows/Linux and then explicitly
for macOS: "Well macOS too for this release, I just don't want the duplicate
things you mentioned or misconfigured accessibility things (all that you
mentioned)".

## Decision

For product version **4.0.0 only**, on all six supported targets, use frame-work
p95 at most 100 ms, input p95 at most 100 ms, and ordinary-refresh maximum at
most 100 ms. Preserve the original frame/input verdicts in every report and
record the applied policy and limits separately. Later versions default to the
original targets. This decision does not convert a failed historical report
into an original-target pass.

The macOS passive-focus publication correction and guarded legacy AppKit
accessibility dispatch remain required. Native fixtures must verify that
snapshot publication does not emit assistive actions, real external focus and
supported text/selection actions still work, and unsupported selectors do not
throw. Both Intel and Apple Silicon must execute these checks. Timing lenience
does not waive duplicate input, dispatch errors, incomplete measurements,
failed shutdown, or functional regressions.

All CPU, RSS/private-memory, collection, correctness, security, lifecycle,
two-hour soak, six-target build, and public-release verification requirements
remain unchanged. Resource windows remain unobserved and separate from the
instrumented interaction runs.

## Follow-up and verification

Codex owns task #r16 to restore the original responsiveness targets after v4,
using retained native reports to select useful optimizations. Policy fixtures
cover all operating systems, future-version expiry, missing observations and
each 100 ms boundary. The runtime harness still fails on functional exceptions
and clean-shutdown failure. Native CI is the final runtime oracle.
