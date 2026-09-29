# ADR 0025: release acceptance for the 4.1.0 overview

Date: 2026-09-29
Status: Accepted by the operator for 4.1.0 only
Related: ADRs 0019, 0021, 0023 and 0024; tasks #ov6, #r16, #r17 and #ext

## Decision

After the remaining processor-use, native timing and physical-display checks
were presented as a choice between continued blocking and a version-specific
exception, the operator instructed: "Go ahead and deploy and ship it to
production, release the new version update."

Accept the documented performance overruns and defer the remaining full resource
matrix and physical 200% display inspection for exactly 4.1.0. This replaces the
pending gate-owner decision; it does not turn a failed benchmark into a pass.
Keep numeric thresholds, raw reports, workloads and their failed verdicts intact.
The original engineering goals remain owned by Codex in #r16 and #r17, with
physical scaling coverage in #ext. No exception applies to prereleases, 4.1.1,
later versions, unknown platforms or functional failures.

## Evidence and limits

The production Windows ce6b42e overview was inspected at the default size and
then measured without observers for 330.047 seconds: CPU 2.528% of one core
fails the original 2% goal; RSS 132.51 MiB and private memory 260.41 MiB pass.
Shutdown is clean. Earlier native Windows inspection passed at physical 150%
scaling; the monitor does not offer standard 200% scaling. Reference raster
tests are not a substitute for that physical inspection.

CI 36371183045 on 0f424c6 completes all twelve interaction cohorts and clean
shutdown on every native target. Original timing passes on Windows, GNU ARM64
and musl x86-64. GNU x86-64 and both Macs fail frame p95; Intel Mac also fails
input p95. Original limits stay 16.7 ms frame p95, 50 ms input p95 and 100 ms
ordinary-refresh maximum. Windows installer qualification 36369836717 passes
on 4ea5278, before the final status-font adjustment. The final source must be
qualified again before merging.

## Enforcement

The interaction report retains `passed=false` and `timing_passed=false` for
timing failures. A separate `release_accepted` field with
`release_exception=ADR-0025-overview-4.1.0` permits a successful qualification
command only when all expected cohorts complete and shutdown is clean, without
functional exceptions. Tests cover missing cohorts, duplicate cohorts, failed
shutdown, functional errors, unknown platforms and expiration.

Resource report verdicts and resource policy remain unchanged. The deferred
full resource matrix is explicitly incomplete, not silently accepted by a
modified resource gate. Existing foreground/hidden functional smoke checks
still run, separately from resource acceptance.

Exact-source CI under ADR 0019, all native functional checks, composite
installer/update/uninstall lifecycles, signing, artifact identity, checksums,
attestations and public-byte verification remain mandatory. Publication uses
the normal main-branch release chain; no local or premature publication.
