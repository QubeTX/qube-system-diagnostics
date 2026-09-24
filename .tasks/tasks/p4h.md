TT;DR: Correct silent PowerShell helper exits found while verifying the actual public Windows installation. Preserve immutable 4.0.0 artifacts and the user's working public installation.

## Why

Both Windows PowerShell and PowerShell 7 exit successfully without executing a command when launched detached with null stdin on the operator's machine. The public automatic updater receives empty JSON. Eight direct runtime cases isolate DETACHED_PROCESS from CREATE_NO_WINDOW, independently of Rust capture code and network access.

## Scope

Use a hidden console only for PowerShell hosts. Preserve suspended job ownership, bounded output/deadlines/cancellation and detached native monitoring workers. Add real Windows-host regression coverage for file/pipe capture and nonzero command outcomes. Document the separate immutable 3.x updater's ABI-1 check, which rejects the ABI-2 v4 companion and requires the official installer for this major upgrade.

## Plan

Validate the focused helper fix locally, push for hosted native qualification, retain public installation proof and publish accurate recovery instructions. A corrective 4.0.1 would be a second release and the original 4.0.0 performance exceptions do not automatically extend to it; obtain the operator's explicit decision before changing that scope.

## Acceptance

Real PowerShell commands execute with redirected handles, retain output/exit codes, and remain bounded. Native workers retain console-free behavior. Public update checks succeed after any corrective publication. Both frontends and existing install ownership/settings remain intact.

## Verification

- [x] Reproduce failure on immutable public 4.0.0 and isolate both PowerShell hosts with direct launch-flag comparison
- [x] Root suite: 231 library tests, eight CLI compatibility tests and three worker integration tests pass; child-only/native opt-in fixtures retain documented skips
- [x] Separate engine suite: 17 tests pass; formatting and clippy pass
- [x] Real PowerShell 5.1/7 file and pipe capture, command execution and nonzero exits pass after correction
- [ ] Hosted qualification of the correction
- [ ] Operator decision on a second release and its performance-policy scope
- [ ] Verify public corrected bytes and actual local update route after any publication

## Status

ACTIVE, owner Codex. Source correction prepared after public 4.0.0 verification. User's installed CLI/GUI/engine match public artifacts, the GUI launches, and settings are preserved. The correction is not installed locally or published; the actual public copy remains 4.0.0.

## Activity

- 2026-09-23 — codex: pushed e5b7b08 in PR #11 and dispatched native CI 35938397261. macOS/Linux root and security checks pass; remaining native jobs are running. Website upgrade guidance PR #17 merged as a63b90c, production 6627424406 succeeds, and actual desktop/mobile gallery plus upgrade-copy checks pass. Installed public payload hashes and saved settings remain unchanged; GUI is open on the actual public app. Next: operator decision and completion of hosted checks before any corrective publication.

- 2026-09-23 — codex: public install verification isolated the failure; real PowerShell regression and full root/engine suites pass. Requested explicit approval for a 4.0.1 corrective release with the same v4 ceilings because it changes both the one-release plan and version-scoped exceptions. Next oracle is hosted native CI.
