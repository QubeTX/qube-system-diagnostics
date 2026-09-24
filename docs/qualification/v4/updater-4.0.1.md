# SD-300 4.0.1 updater correction

Recorded 2026-09-23 (Central time). Publication and installed-copy checks are
pending; this document must be completed before closing task #p4h.

## Installer discovery audit (2026-09-24)

The operator reports a separate work PC installed through PowerShell but has no
discoverable desktop app or `sd300` command even after reopening PowerShell. That
host and its security logs are unavailable; WatchGuard blocking is a hypothesis,
not a diagnosis. The available Windows host has a valid public 4.0.0 CLI, saved
user PATH, GUI payload and Start-menu entry. These are separate observations.

Code review found a hard-coded Programs path instead of the configured Windows
known folder, no final shortcut/PATH verification, no installing-process PATH
refresh, and disagreement about custom-prefix environment variables between the
wrapper and cargo-dist. The candidate now uses the configured Programs folder for
installation and removal, reads back target/working directory/icon, verifies saved
user PATH, preserves explicit PATH opt-outs and refreshes the installing process.
Child destination, backup and rollback now agree. Unmanaged mode is rejected
before mutation because this wrapper requires a receipt. Failures name the stage
and report restoration separately rather than promising that every failure was safe.

Safe fixtures create real COM shortcuts only in a unique temporary directory and
exercise child execution on Windows PowerShell 5.1 and PowerShell 7. Persistent
PATH is mocked, and the real saved user PATH is checked unchanged. Root tests,
clippy, release build, package dry-run and the separately scoped engine tests pass.
Hosted installer and cross-platform results for this expanded candidate are pending;
earlier d8a6f87 reports below are not proof of these later installer changes.

Windows folder redirection is documented by [Microsoft](https://learn.microsoft.com/en-us/windows/win32/shell/known-folders).
Process and persistent environment scopes are documented in
[PowerShell's environment reference](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_environment_variables).

The requested Linux audit found the same custom-prefix disagreement plus shell
startup gaps when `.bashrc` is absent or fish uses a relocated `XDG_CONFIG_HOME`.
The wrapper now creates only the missing Bash startup integration, adds fish's
source line in its actual configuration directory, and tracks exact profile
contents and newly created directories for rollback. Existing profiles and
concurrent edits are preserved; explicit PATH opt-outs remain authoritative.
These shell changes also apply to the managed macOS installer. Linux desktop
entry path, executable quoting and custom-icon registration were already present.

Local shell fixtures use Git Bash with isolated home directories, not a native
Linux desktop. Native CI now installs fish and requires its fresh-shell lookup;
the Linux composite lifecycle now enables PATH integration and invokes the actual
installed command from a fresh Bash session. Previously it always opted out of
PATH and used an absolute executable path, which could not detect this symptom.
Native evidence must be recorded separately from the local fixture results.

Final local installer fixtures: 33 checks pass on each of Windows PowerShell
5.1.26100.9444 and PowerShell 7.6.6. The shell suite runs 19 cases under Git Bash:
18 pass and native fish is explicitly skipped. It covers a fresh interactive Bash,
literal path quoting, custom-prefix precedence, child/parent environment isolation,
PATH opt-outs, idempotence, failure propagation, exact rollback and preservation of
concurrent profile edits. A temporary selected-bin PATH entry is removed only from
the non-opt-out child environment so cargo-dist cannot mistake it for persistent
integration; all other entries and the parent PATH are preserved.

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
