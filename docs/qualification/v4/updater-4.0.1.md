# SD-300 4.0.1 updater correction

Recorded 2026-09-23 (Central time). Publication and installed-copy checks are
pending; this document must be completed before closing task #p4h.

## Reproduced failures and correction

Public Windows 4.0.0 failed its automatic release check on the operator's actual
managed installation. Eight direct process-launch cases isolated a successful
but empty exit in both Windows PowerShell 5.1 and PowerShell 7 when detached
with null stdin. Hidden-window launch executed the same commands through both
file and pipe capture. Only PowerShell hosts change their launch flags; native
monitoring workers retain their detached launch and owned-job cleanup.

The first live release check after that correction exposed a separate encoding
failure in GitHub release metadata. Explicit UTF-8 output fixed the real public
response. HTTP failures now exit nonzero, and each transport must return a valid
stable-release response before it can succeed. Empty, malformed, incomplete,
invalid-encoding and unsupported-version responses retain bounded diagnostics
and try the next transport. Errors identify the release-check step, explain
that installation files were not changed at this stage, and give recovery steps.

The separate immutable 3.1.3 updater imposed its own private engine ABI on the
new GUI. Future updates validate the stable self-test envelope, product/version
identity and the new GUI's successful validation of its own bundled engine.
Same-version repairs still enforce this version's exact engine contract.

Neither fix can alter an older executable already installed on a customer's
computer. Windows 4.0.0 and affected 3.x installations need their matching
official installer once. No uninstall or silent owner change is required.

## Candidate evidence

[PR #11](https://github.com/QubeTX/qube-system-diagnostics/pull/11) candidate
`d8a6f87b4916d8aaa8177e8dad19d18514c6497b` passes:

- 235 root library tests, eight CLI compatibility tests and three real worker
  tests; documented opt-in and platform fixtures retain their explicit skips.
- 17 separately scoped engine tests, formatting, clippy, release CLI build and
  package-publication dry-run.
- 72 native GUI tests with two platform skips, strict model checks, coordinated
  version checks, release GUI build, distribution/path-leak checks and an actual
  Windows self-test reporting product 4.0.1 with ABI/schema 2.
- Real PowerShell 5.1/7 file/pipe execution, stdout/stderr and nonzero-exit tests.
  A real public GitHub release check passes locally and on hosted Windows,
  bypassing the synthetic release override that masked the original defect.
- [Windows composite qualification](https://github.com/QubeTX/qube-system-diagnostics/actions/runs/35944422700):
  all four native installer channels, updates, takeover, rollback/commit,
  uninstall, immutable-v2 transitions and managed CLI/GUI completion.

[Native CI](https://github.com/QubeTX/qube-system-diagnostics/actions/runs/35944427064)
first-pass results retain all original verdicts. Windows, both Macs and GNU
ARM64 pass. GNU x86-64 has compact Technician navigation input p95 117.463 ms;
musl has wide Technician navigation input p95 198.642 ms. Both fail the approved
100 ms input gate despite passing frame-work/refresh gates and clean shutdown.
The failed reports and job logs are retained. Attempt 2 repeats only those two
jobs on fresh runners with the unchanged candidate. GNU again fails the input
gate: wide Technician navigation p95 875.864 ms, compact Technician navigation
450.717 ms and compact Technician keyboard 425.557 ms. Its frame p95 remains at
or below 18.361 ms and refresh maximum at or below 8.464 ms; all expected inputs
complete and shutdown is clean. The musl repeat passes with worst input p95
34.714 ms. Retry reports were fetched by their exact artifact IDs
`10786918994` (GNU) and `10786419374` (musl), because duplicate artifact names
from both attempts remain available. No third blind repeat is planned.
A specific operator decision is pending on whether to hold
this updater-only correction for the separate Linux GUI latency investigation
or release with that failure documented; the current release gate is not waived.

All eight per-target/attempt summaries, original verdicts and exact executable
hashes are retained in [the native interaction record](native-interaction-updater-d8a6f87.json).
The GNU runtime smoke before the repeat reports clean shutdown with no remaining
owned processes. GTK scheduling and automation-observer costs remain hypotheses,
not established explanations. No pinned SDK cache was edited during inspection.

The original resource/soak evidence remains identified by its v4.0.0 artifact
hashes. This updater patch does not claim a new two-hour soak. ADR 0018 defines
the corrective-release scope; all original future targets remain in
[Next-version targets](../../next-version-targets.md).

## Public and local verification

Pending publication, exact public asset/checksum/attestation checks, real
production update checks, same-owner local installation with settings
preservation, and production website verification. No candidate binary is
copied over the operator's public installation as a substitute for these checks.
