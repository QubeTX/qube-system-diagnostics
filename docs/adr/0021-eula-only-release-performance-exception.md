# ADR 0021: performance acceptance exception for the 4.0.2 EULA correction

Date: 2026-09-24
Status: Accepted by the operator for 4.0.2 only
Related: ADRs 0019-0020; tasks #eul, #r16 and #r17

## Decision

The operator requested the real PolyForm Noncommercial 1.0.0 license in the
Windows MSI EULA, authorized a corrective release, and excluded performance
work. After the publication blocker was explained and a **4.0.2-only release
exception preserving thresholds and recorded failures** was requested, the
operator explicitly answered: "Yes, get it deployed, please."

Accept performance evidence as nonblocking for this EULA-only release. Do not
change the runtime, benchmark workloads, measurements, numeric thresholds or
the failed verdicts. Original targets remain owned by Codex in #r16 and #r17.
This exception expires after exactly 4.0.2; it does not include prereleases,
4.0.3, later minor releases, or unknown platforms.

## Enforcement and evidence

Candidate 0d27a78864b6ac8b89ee36b5289f74db9a75bbbd passed the complete Windows
installer lifecycle in run 36062830554. Global, Corporate and compatibility
MSIs each embedded the complete license and rendered 4357 characters through
Windows RichEdit. CI 36062833833 passed core tests on all three platforms,
security and release planning, but all six native targets failed timing.
Retain that run and its raw reports; do not relabel them as performance passes.

The interaction report retains `passed=false`, `timing_passed=false` and all
original measurements when timing fails. A separate `release_accepted` field
and explicit `release_exception=ADR-0021-eula-4.0.2` allow the qualification
command to succeed only when every expected interaction cohort completes,
shutdown is clean, and there is no functional exception. Missing evidence,
input loss, crashes, timeouts and shutdown failures remain blocking. Tests
cover that boundary and the expiration of the version-scoped exception.

ADR 0019's newest exact-source CI barrier remains unchanged: publication still
requires green CI for the precise release candidate, with the above explicit
acceptance policy. Signing, installer lifecycle, licensing, artifact identity,
checksums, attestations and public-byte verification remain mandatory.
