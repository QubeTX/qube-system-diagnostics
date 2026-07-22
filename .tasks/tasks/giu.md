TT;DR: Add an Update action to the SD-300 app/tray after v3 ships, without making the running GUI replace itself or creating a second lifecycle implementation. A short-lived verified coordinator should invoke the same proven owner/channel transaction, report failures clearly, and relaunch the app only after the CLI and GUI verify together.

## Why
The operator asked whether SD-300 could provide the same nontechnical update experience being designed for Goose: initiate an update from the app or tray, update both the CLI/TUI and GUI, then reopen the updated app. This is valuable, but it is deliberately deferred from v3.0.0 because the approved v3 contract keeps lifecycle actions status-only in the GUI and the current release already has a large six-target installer/update/uninstall matrix.

The design reference is the active Codex task titled "I'm on an admin account..." in `C:\Users\hey\git\goose` (thread `019f7c6f-93fc-7d91-9554-3ffded4a358b`). Its current investigation concluded that the tray process must not update itself: the menu action should start a separate coordinator, let the app exit, run the proven updater/elevation path, persist an inspectable result, and relaunch only after verification.

## Plan
1. Recheck the completed Goose implementation/evidence when that work lands; reuse its lifecycle lessons without copying product-specific ownership assumptions.
2. Add an app/tray Update action backed by a version check whose disabled/available/error states never block opening the monitor.
3. Launch a signed or bundle-owned short-lived coordinator from an absolute application-relative path. Authenticate its request/result channel and prevent arbitrary command or path injection.
4. Have the coordinator invoke SD-300's existing owner-preserving update command/worker rather than reproducing download, checksum, elevation, rollback, receipt, Cargo-migration, MSI/EXE/PKG, or managed-installer logic.
5. Stop the GUI through the existing authenticated lifecycle endpoint, preserve rollback, verify both CLI and companion, record a bounded result, and relaunch the installed app only after success. On failure, leave the proven prior product usable and surface recovery instructions.
6. Qualify Windows UAC and all four native channels, macOS managed/PKG owners, Linux managed owners, Cargo two-step migration, tray/no-tray behavior, running-image replacement, offline/checksum failure, rollback, and relaunch suppression when the user had chosen Quit.

## Impact
Intended: nontechnical users can update the complete SD-300 product without manually opening a terminal. The CLI remains the authoritative lifecycle engine and all owner/channel/scope behavior stays shared.

Possible unintended impact: self-update deadlock, duplicate app instances, privilege confusion, relaunch loops, lost error output, a second updater contract, or a coordinator surviving longer than necessary. The helper must be short-lived, versioned, bundle-owned, least-privileged until the existing channel requires elevation, and covered by the same exact-tag/checksum/rollback rules.

## Acceptance
The app and supported tray menus can initiate an available update without embedding lifecycle mutation in the UI process. The complete CLI+GUI product updates through its proven owner, the old app exits cleanly, success relaunches exactly one verified new app, failure preserves or restores the previous product and presents an actionable result, and terminal-driven `sd300 update` remains unchanged.

## Verification
- [ ] All supported owner/channel update and rollback matrices pass when initiated from the GUI coordinator
- [ ] Success relaunches exactly one verified updated GUI and updates the CLI/TUI to the identical product version
- [ ] Failure, cancellation, offline, checksum, and elevation-refusal cases preserve the proven prior installation and expose an actionable result
- [ ] Existing `sd300 update`, JSON stdout, bare TUI launch, direct installer repair, tray, and uninstall contracts remain compatible

## Status
To-Do, explicitly blocked by final v3 qualification and release task #qv3. The board now attaches this follow-on coordinator to milestone #v3n for visibility, but it remains post-release work: no current v3 product scope or lifecycle contract changed.

## Activity
- 2026-07-21 17:53 — created from the operator's request and deliberately deferred from v3 after inspecting the active Goose updater-coordinator investigation (agent: codex)
- 2026-07-22 00:27 — reconciled the detail with the board index: queued in To-Do behind #qv3 and visible under #v3n, still explicitly post-release rather than a current release blocker (agent: codex)
