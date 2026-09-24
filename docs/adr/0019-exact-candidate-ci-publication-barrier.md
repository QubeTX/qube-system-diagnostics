# ADR 0019: exact-candidate CI before publication

Date: 2026-09-24
Status: Accepted
Related: ADR 0018, tasks #p4h and #r16

## Evidence

The feature candidate 08b7fe5 passed all six native timing gates in CI
36045921364 and Windows composite qualification 36046014017. Its source tree
exactly matches merged release source ae5a8995. The independent CI run started
by that merge, 36050405161, later recorded an Intel Mac ordinary-refresh maximum
of 116.583 ms against the unchanged 100 ms limit. Input p95 was 61.758 ms, frame
p95 52.100 ms, all 171 inputs completed, and shutdown was clean. The other five
targets passed. This additional failure is neither waived nor erased by the
earlier passing run.

That timing step failed at 20:21:29 UTC. The separately triggered release
qualifier published GitHub release 4.0.1 at 20:21:49 UTC, after all its native
installer, draft lifecycle and artifact gates passed. It did not wait for the
concurrent exact-commit CI workflow. Publication cannot rely on the relative
speed of these independent workflows.

## Decision

Before publishing either the crate or GitHub draft, require successful CI for
the exact candidate commit. Select the newest CI run for that SHA from push or
workflow_dispatch events. Do not treat a PR run's head SHA as proof: its actual
checkout is a merge ref. Do not fall back to an older successful run when the
newest run is pending or failed. Canceled, skipped, neutral, timed-out and
action-required runs do not qualify.

Wait at most one hour, with bounded API calls. Missing CI, query errors, a
deadline or any completed non-success result block publication with an explicit
reason. A manual tag release can qualify through CI dispatched at that exact
tag/commit. Already-public artifact rechecks do not republish or change tags.

Keep 4.0.1 immutable and retain both timing matrices. Its Windows update and
installation correction is independently verified against public bytes. The
Intel refresh finding remains in #r16 and the release notes. This workflow fix
does not claim to correct that rendering cost or to grant a timing exception.

## Verification

Nine deterministic cases cover exact revision/event identity, pending-to-success,
pending-to-failure, newest-run precedence, missing-run deadlines and non-success
conclusions. A read-only live invocation against ae5a8995 correctly rejects the
actual failed CI run. The barrier is immediately before the first publishing
step and uses the workflow's existing actions-read permission. No product,
installer, dependency, version or published asset changes are involved.
