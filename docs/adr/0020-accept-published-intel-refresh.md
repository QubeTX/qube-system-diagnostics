# ADR 0020: accept the observed Intel Mac refresh for 4.0.1

Date: 2026-09-24
Status: Accepted by the operator for the published 4.0.1 release only
Related: ADRs 0016-0019, tasks #p4h and #r16

## Decision and scope

After reviewing the measured Intel Mac ordinary-refresh maximum, the operator
said: "And that 116 millisecond refresh, that's totally fine."

Accept the **116.583 ms** observation in merged-source CI 36050405161 as a
release-specific exception for immutable 4.0.1. It no longer blocks acceptance
of that release. This records acceptance of the observed result, not a general
new ceiling or a claim that the renderer was fixed.

Retain the raw failed 100 ms verdict, complete native matrix and exact artifact
identities in [the latency record](../qualification/v4/latency-4.0.1.md).
The run's Intel input p95 is 61.758 ms and frame p95 is 52.100 ms; all 171
interactions complete and shutdown is clean. The other five targets pass.

The original targets in [Next-version targets](../next-version-targets.md)
remain unchanged for versions after 4.0.1, including ordinary refresh at most
100 ms. Codex owns future attribution and improvement in #r16. Do not change
the benchmark, test-policy thresholds, version, published artifacts or tag to
turn the old CI check green. ADR 0019's exact-source publication barrier remains
mandatory; this narrow timing decision does not waive any other failing check.

## Separate later CI failure

Post-release CI 36056348704 on 430e6c1 passes all six native GUI jobs. Its failing
Windows step is the live public release request: both PowerShell transports
receive HTTP 403. The log lacks response headers, so the historical cause cannot
be distinguished between a quota limit and another HTTP rejection. This is a
separate investigation, not covered by the timing exception.
