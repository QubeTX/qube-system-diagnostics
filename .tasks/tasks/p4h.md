TT;DR: Correct silent PowerShell helper exits found while verifying the actual public Windows installation. Preserve immutable 4.0.0 artifacts and the user's working public installation.

## Why

Both Windows PowerShell and PowerShell 7 exit successfully without executing a command when launched detached with null stdin on the operator's machine. The public automatic updater receives empty JSON. Eight direct runtime cases isolate DETACHED_PROCESS from CREATE_NO_WINDOW, independently of Rust capture code and network access.

## Scope

2026-09-24 extension: audit the operator's separate work-PC report of no discoverable GUI and no command after restarting PowerShell. Confirm composite install behavior; repair Windows known-folder discovery, persistent/session PATH checks, custom-prefix consistency and actionable failure output. Endpoint blocking is unconfirmed; this host's public installation is healthy apart from the known updater defect.

Use a hidden console only for PowerShell hosts. Preserve suspended job ownership, bounded output/deadlines/cancellation and detached native monitoring workers. Add real Windows-host regression coverage for file/pipe capture and nonzero command outcomes. Document the separate immutable 3.x updater's ABI-1 check, which rejects the ABI-2 v4 companion and requires the official installer for this major upgrade.

## Plan

Publish the operator-authorized 4.0.1 correction after native and composite qualification. The 2026-09-23 instruction explicitly authorizes push, merge and deployment as an update, followed by installation on the operator's machines. ADR 0018 records the same v4 ceilings for this corrective patch, with original limits restored afterward. Verify the actual public release-check and already-current route; retain old-version recovery instructions. This Windows machine is identified; names of any additional machines have been requested.

## Acceptance

Real PowerShell commands execute with redirected handles, retain output/exit codes, and remain bounded. Native workers retain console-free behavior. Public update checks succeed after any corrective publication. Both frontends and existing install ownership/settings remain intact.

## Verification

- [x] Windows discovery/PATH fixtures pass on PowerShell 5.1 and 7 without modifying the actual installation or persistent user PATH
- [x] Hosted composite installer matrix qualifies the redirected-discovery/PATH changes (36034296828, d18a4e5)
- [x] Native Linux Bash/fish startup fixtures prove reopened-shell command discovery (19 tests, 36034285985)
- [ ] Actual Linux composite archive install proves reopened-shell command discovery before publication

- [x] Reproduce failure on immutable public 4.0.0 and isolate both PowerShell hosts with direct launch-flag comparison
- [x] Root suite: 235 library tests, eight CLI compatibility tests and three worker integration tests pass; child-only/native opt-in fixtures retain documented skips
- [x] Separate engine suite: 17 tests pass; formatting and clippy pass
- [x] Real PowerShell 5.1/7 file and pipe capture, command execution and nonzero exits pass after correction
- [ ] Hosted qualification of the correction
- [x] Operator authorized the corrective update after the explicit 4.0.1/same-v4-ceilings proposal; ADR 0018 records scope
- [ ] Verify public corrected bytes and actual local update route after any publication

## Status

ACTIVE, owner Codex. Source correction prepared after public 4.0.0 verification. User's installed CLI/GUI/engine match public artifacts, the GUI launches, and settings are preserved. The correction is authorized and being qualified as 4.0.1. It is not installed locally or published yet; the actual public copy remains 4.0.0.

## Activity

- 2026-09-24 — codex: d18a4e5 Windows composite matrix 36034296828 PASSES in full. CI 36034285985 completes with Windows and all three Linux GUI targets passing, native Linux Bash/fish 19/19 passing, and macOS shell fixtures passing. Preserve Apple Silicon refresh maximum 130.670 ms and Intel input p95 100.875 ms as failed 100 ms gates; both shut down cleanly and retain all functional results. Saved all six reports with exact artifact IDs. Follow-up 9d0adc5 changes only successful test completion reporting and passes the exact GitHub wrapper on both local PowerShell runtimes. Push this correction with evidence for the next native oracle; no threshold or product rendering changes. Website discovery guidance b88992c passes lint/build and desktop/mobile browser checks, held in PR #18 for app publication.

- 2026-09-24 — codex: pushed installer candidate d18a4e5 and started CI 36034285985 plus unpublished Windows matrix 36034296828. Native Ubuntu and macOS shell fixtures pass, including required real fish on Ubuntu. Windows hosted fixtures pass all 33 assertions but GitHub's wrapper propagates the deliberately exercised child exit; reproduced the exact wrapper locally and corrected only test completion status. Product/GUI candidate is unchanged; preserve the running matrix and native timing results before the next push.

- 2026-09-24 — codex: final shell suite passes 18/19 locally, with native fish explicitly skipped on Windows; CI now requires fish on Linux. Corrected the child PATH shortcut so a temporary bin entry cannot suppress persistent setup, without changing the parent environment or explicit opt-outs. Linux composite qualification now opens a fresh Bash session after a real PATH-enabled install. Preparing one combined installer candidate for hosted qualification; earlier performance failures remain recorded and are not waived.

- 2026-09-24 — codex: source audit confirms composite GUI delivery already exists on all supported platforms. This host has a working public GUI shortcut and persistent PATH; the work PC is separate and unavailable. Corrected hard-coded Programs location, missing shortcut readback/current-shell PATH refresh, custom-prefix mismatch, and rollback PATH attribution. Added persistent-PATH verification, explicit custom-icon registration and phase-specific failures. Both real PowerShell hosts pass 33 safe fixtures; root 235 tests, eight CLI tests, three workers, 17 engine tests, clippy, release build and package dry-run pass. The requested delegated Linux audit found additional fresh-Bash/custom-fish startup gaps and repaired them with rollback fixtures. Native installer and startup qualification are next; no WatchGuard cause or public corrective release is claimed.

- 2026-09-23 — codex: musl attempt 2 PASSES (worst input p95 34.714 ms), leaving only GNU x86-64 input timing unresolved. Verified repeat reports by artifact ID 10786419374 (musl) and 10786918994 (GNU); downloading by shared artifact name can return the prior attempt. Both first and repeat results remain saved, and no third blind repeat is scheduled. Publication still awaits the specific gate decision or a demonstrated fix.

- 2026-09-23 — codex: unchanged-candidate GNU repeat reproduces the input failure (875.864 ms wide Technician navigation; 450.717 ms compact navigation; 425.557 ms compact keyboard). Frame p95 stays at or below 18.361 ms, refresh at or below 8.464 ms, expected inputs complete and shutdown is clean. Saved the second report and stopped the retry loop. Requested the gate owner's explicit choice between holding for separate Linux GUI work and shipping this fully qualified Windows updater correction with the Linux failure documented. No exception is assumed and publication remains held; musl's existing repeat continues. Read-only follow-up is checking GTK frame scheduling and automation observation paths without modifying the pinned SDK cache.

- 2026-09-23 — codex: Windows composite qualification 35944422700 PASSED in full, including immutable v2 transitions, legacy rollback/commit and managed completion. Both macOS architectures and Windows/GNU ARM64 native CI pass. First-pass GNU x86-64 input p95 is 117.463 ms; musl has one 198.642 ms navigation cohort, with other input cohorts at or below 31.215 ms, frame p95 at or below 20.374 ms, refresh maximum at or below 6.099 ms and clean shutdown. Both failed reports/logs are retained. Requested one fresh-runner repeat of only these two failed jobs on unchanged d8a6f87 (CI 35944427064 attempt 2). No release or product code change has been made to conceal their verdicts.

- 2026-09-23 — codex: d8a6f87 local release GUI build, strict/native tests and live self-test pass (4.0.1, ABI/schema 2); real GitHub release transport also passes on hosted Windows. CI 35944427064 passes root Windows/macOS/Linux, security, dist plan, Windows GUI and GNU ARM64 GUI. GNU x86-64 retains a 117.463 ms navigation-input p95 failure at compact Technician size; frame work and refresh gates pass with clean shutdown. Saved its report and job log before any retry. Next falsifiable check is one unchanged-candidate fresh-runner repeat after the remaining native jobs finish; do not alter rendering or relax the accepted 100 ms gate. Windows lifecycle 35944422700 has passed initial installs and reached immutable-v2 transitions. Website PR #18 is validated and held for actual publication.

- 2026-09-23 — codex: 4.0.1 passes 235 library tests, eight CLI tests, three real worker tests, 17 engine tests, formatting/clippy, release build, package dry-run, eight resource-policy and eleven interaction-policy fixtures. Live public release transport now returns v4.0.0 correctly. A stale local npm SDK copy failed the reviewed hash guard; restored it through locked npm ci, with no pin or cache edits, and restarted native tests. The preceding 161799b CI passes Windows/GNU/Apple Silicon but retains Intel and musl input-timing failures; preserve reports and use the new exact candidate as the next native oracle rather than changing unrelated rendering.

- 2026-09-23 — codex: operator reports the vague failure directly and authorizes investigate/fix/push/merge/deploy plus local installation. Reproduced public 4.0.0 empty output. The first live test of corrected launch flags exposed a second encoding failure; explicit UTF-8 now passes the real GitHub API. Added bounded fallback/error fixtures, stable companion self-test validation across future private ABI changes and an opt-in real-network Windows CI gate. Source coordinated at unused 4.0.1. Next oracle: current root/engine/GUI tests, six native CI and the composite Windows installer matrix.

- 2026-09-23 — codex: pushed e5b7b08 in PR #11 and dispatched native CI 35938397261. macOS/Linux root and security checks pass; remaining native jobs are running. Website upgrade guidance PR #17 merged as a63b90c, production 6627424406 succeeds, and actual desktop/mobile gallery plus upgrade-copy checks pass. Installed public payload hashes and saved settings remain unchanged; GUI is open on the actual public app. Next: operator decision and completion of hosted checks before any corrective publication.

- 2026-09-23 — codex: public install verification isolated the failure; real PowerShell regression and full root/engine suites pass. Requested explicit approval for a 4.0.1 corrective release with the same v4 ceilings because it changes both the one-release plan and version-scoped exceptions. Next oracle is hosted native CI.
