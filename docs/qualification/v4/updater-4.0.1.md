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

The expanded discovery candidate `d18a4e58a0a690aaf15c741d4baede77b72758b9`
passes [the complete Windows composite matrix](https://github.com/QubeTX/qube-system-diagnostics/actions/runs/36034296828),
including all four native channels, same-channel upgrades, legacy rollback/commit,
immutable-v2 transitions and managed CLI/GUI completion. In
[native CI](https://github.com/QubeTX/qube-system-diagnostics/actions/runs/36034285985),
Ubuntu passes all 19 shell discovery cases with real fish required; macOS also
passes its shell suite. Windows passes all 33 discovery assertions on PowerShell
5.1, but GitHub's wrapper then propagates the intentionally exercised child exit.
The exact wrapper reproduces that reporting failure locally. A test-only correction
sets success after assertions and cleanup; both PowerShell hosts then return zero
under the same wrapper. Assertion/cleanup exceptions still terminate before that
success assignment. This is separate from the product updater defect.
Follow-up CI `36037050150` confirms both PowerShell 5.1 and 7 discovery steps pass
on the hosted Windows runtime after this correction.

The final bounded shell review found an inconsistent directory-link contract:
backup accepted existing configuration-directory symlinks while capture rejected
them. Custom XDG roots now encounter that pre-existing limitation too. The follow-up
records and verifies link identity, preserves original linked directories during
rollback, and rejects new or changed links before profile completion/capture.
Rollback skips a changed linked subtree rather than modifying its replacement.
Five native symlink cases supplement the earlier shell fixtures. Their Windows
skip is explicit because this filesystem cannot create the links; Ubuntu/macOS
must exercise them before publication.

The discovery candidate's native timing reports are retained in
[a separate record](native-interaction-discovery-d18a4e5.json). GNU x86-64 passes
with worst input p95 18.082 ms, frame p95 12.414 ms and refresh maximum 6.534 ms.
This does not establish why its earlier unchanged-GUI runs were slower. Apple
Silicon has one 130.670 ms refresh maximum, with 121.935 ms maximum native drawing;
input/frame p95 pass the accepted ceilings and shutdown is clean. Retain that
failed refresh verdict when qualifying the test-only follow-up. Intel Mac input
p95 is 100.875 ms, narrowly over the same 100 ms gate; frame p95 is 54.155 ms and
refresh maximum 62.790 ms, with clean shutdown. Windows and all three Linux
native GUI targets pass. No drawing or input code changed in this installer
candidate, and no new performance exception is assumed.

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

### Final discovery candidate: efbab9e

[CI 36037699515](https://github.com/QubeTX/qube-system-diagnostics/actions/runs/36037699515)
finishes with all root Windows/macOS/Linux jobs, security and packaging plan
passing. Native Ubuntu runs **24/24** shell discovery/rollback cases, including
real fish and all five directory-symlink cases. Native macOS runs 24 cases with
23 passing and only fish skipped. Both hosted PowerShell runtimes pass the 33
Windows discovery assertions and return successful suite status. The expanded
Windows composite lifecycle remains qualified by run `36034296828`; the final
follow-up changes no Windows product or installer source.

All six native GUI targets complete their functional checks and clean shutdown.
Final timing results below use the approved 100 ms ceilings; an input or refresh
failure still fails qualification. The complete retained report is
[native-interaction-discovery-efbab9e.json](native-interaction-discovery-efbab9e.json).

| Target | Worst input p95 | Worst frame p95 | Worst ordinary refresh | Timing verdict |
| --- | ---: | ---: | ---: | --- |
| Windows x86-64 | 44.093 ms | 18.447 ms | 7.171 ms | Pass |
| macOS Apple Silicon | 35.990 ms | 34.232 ms | 97.786 ms | Pass |
| macOS Intel | 104.133 ms | 50.630 ms | 23.001 ms | Fail: input |
| Linux GNU x86-64 | 37.222 ms | 23.341 ms | 8.993 ms | Pass |
| Linux GNU ARM64 | 27.113 ms | 17.656 ms | 6.224 ms | Pass |
| Linux musl x86-64 | 498.065 ms | 16.956 ms | 4.852 ms | Fail: input |

The musl wide Technician keyboard trace contains six consecutive delayed inputs
with latencies from 115.605 to 666.377 ms; each advances the input count exactly
once. Runtime-uptime and frame deltas corroborate a real response delay rather
than a percentile arithmetic error. The affected cohort's frame p95 is 7.608 ms
and maximum synchronous frame work is 10.488 ms. Attribution to event scheduling,
collector contention or runner load remains unproved. Do not label it runner
noise, change the measurement interval, or run a third blind repetition.

The earlier Mac-only exception question is superseded by the final Intel/musl
measurements. An explicit owner decision is pending between publishing this
installer/updater correction with documented GUI timing deferrals and holding
publication for a separate GUI latency fix. Existing publication authorization
does not waive the numeric gate. The production release and local installation
remain immutable 4.0.0; website PR #18 remains held. The final evidence is saved on
`codex/sd300-401-qualification-evidence` without starting another unchanged GUI CI
run on product PR #11.

Pending publication, exact public asset/checksum/attestation checks, real
production update checks, same-owner local installation with settings
preservation, and production website verification. No candidate binary is
copied over the operator's public installation as a substitute for these checks.
