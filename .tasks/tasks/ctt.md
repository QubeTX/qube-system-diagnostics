TT;DR: Bundle a first-class close-to-tray preference and live tray hover summary into the same next SD-300 patch update as the replacement application/tray identity.

## Why
The operator requires a tray-enabled GUI session to keep collecting after the visible window is closed. Today SD-300 implicitly hides on Windows/macOS when tray is enabled, but the behavior is not an explicit user preference and the tray tooltip is only the static product name. The next update must make the lifecycle intentional, configurable, and useful without weakening tray Quit.

## Scope
In scope:

- Add `gui.close_to_tray` with a migration-safe default of `true`.
- Apply the preference only when the current platform supports a tray and `gui.tray_enabled` is effective.
- Keep every bare/flagged TUI launch tray-free; only the separate GUI process may create a tray item.
- With close-to-tray enabled, clicking the window X hides the GUI while the tray monitor stays alive.
- With close-to-tray disabled, clicking X gracefully quits the GUI process and removes the tray item.
- Keep tray Open and Quit commands working in both lifecycle modes.
- Publish a bounded, frequently refreshed hardware summary in the Windows/macOS tray hover tooltip.
- Carry the feature through the reviewed Native SDK patch, settings UI, tests, changelogs, and the same next patch release as #n7k.

Out of scope:

- Adding a Linux tray while Native SDK 0.5.4 does not support one.
- Changing CLI/TUI behavior or settings defaults.
- Publishing a separate lifecycle-only release.

## Plan
1. Extend the GUI settings document and declarative Settings surface with a close-to-tray toggle that defaults on.
2. Separate “tray exists this session” from “window close keeps the tray session alive” in model state and close-event handling.
3. Extend the reviewed Native SDK downstream patch so status-item refresh state can update a bounded tooltip on Windows and macOS.
4. Build the tooltip from the latest bounded CPU, memory, GPU, storage, and health summary without blocking the UI thread or exposing sensitive data.
5. Add model/settings tests plus Native SDK Windows/macOS/null-platform contract tests.
6. Verify close, reopen, update, Quit, toggle/restart, and tooltip behavior together with #n7k packaging and update-lifecycle qualification.

## Impact
Intended:

- Closing a tray-enabled SD-300 window keeps background monitoring available by default.
- Users who want X to terminate the entire app can opt into that behavior explicitly.
- Hovering the tray icon gives a concise live hardware snapshot without reopening the window.

Possible unintended:

- A stale session-time setting can disagree with the “restart required” contract if the toggle is applied immediately in only part of the lifecycle.
- An overlong Windows tooltip can be truncated or rejected by the shell.
- Updating tray state too frequently can add unnecessary shell traffic.
- A platform-specific close path can leave an invisible process or remove a tray item prematurely.

## Acceptance
**Functional bar:** On Windows and macOS, tray enabled plus close-to-tray enabled keeps the tray and collector alive after X; Open restores the singleton window; Quit exits. Disabling close-to-tray makes X exit and removes the tray. Hover text presents a current bounded hardware summary.

Bare `sd300` and all established TUI modes never create or launch a tray item. GUI tray state remains controlled only by GUI settings.

**Evidence bar:** Settings serialization/migration tests, close-policy model tests, Native SDK platform-contract tests, programmatic tray command dispatch, local Windows runtime inspection, hosted macOS build/package proof, and manual operator confirmation of the Windows tray tooltip and close behavior.

**Release coupling:** #ctt and #n7k are both required for the same next patch update. Neither task is release-complete until the combined candidate passes install/update/uninstall qualification.

## Verification
- [x] New and existing settings documents resolve `close_to_tray` correctly without changing TUI defaults.
- [x] Tray-enabled close keeps the process and tray alive by default; Open restores; Quit dispatch maps to graceful application exit.
- [x] Disabling close-to-tray makes X terminate the GUI and tray cleanly.
- [ ] Windows and macOS receive a bounded live tooltip; Linux remains explicitly tray-unavailable.
- [x] Bare and mode-specific TUI launches never create the GUI process or tray item.
- [ ] The combined #ctt/#n7k candidate passes GUI tests, strict checks, Windows runtime acceptance, package manifests, and next-patch update qualification.

## Status
ACTIVE — `gui.close_to_tray` is implemented with an additive default of true, GUI tray presence defaults on, and the startup-effective tray state remains distinct from the persisted next-launch preference. The staged v3.1.2 Windows candidate passed both real close paths: default X kept the process alive, singleton Open restored the hidden window, and the opt-out made X terminate it. The release CLI help/snapshot routes launched no GUI process, and programmatic tray Open/Quit mapping remains green. Manual Windows tray icon/hover/Quit confirmation, hosted macOS proof, and combined synthetic-prior update qualification remain open.

## Activity
- 2026-07-23 01:56 — created as a new Active next-patch requirement and explicitly coupled to #n7k after the operator required configurable close-to-tray persistence plus live hardware hover information (agent: codex)
- 2026-07-23 01:57 — recorded the operator's explicit frontend boundary: TUI launches are always tray-free; only GUI settings may enable or disable the GUI-owned tray (agent: codex)
- 2026-07-23 04:31 — implemented the migration-safe GUI-only defaults (`tray_enabled=true`, `close_to_tray=true`), the Settings toggle, session-effective close policy, and regression tests proving tray-off/close-to-tray-off paths quit while tray-enabled close stays alive (agent: codex)
- 2026-07-23 04:45 — extended the reviewed Native SDK status-item contract with bounded live tooltip updates on Windows/macOS plus null-platform tests; tooltip content now summarizes CPU, memory, GPU, storage, and disk health without sensitive fields (agent: codex)
- 2026-07-23 04:58 — full release-target Windows Native suite passed 37/39 with two expected skips after separating executable-only Win32 resources from the SDK analysis object; installed tray Open/Quit/hover and manual appearance checks remain open (agent: codex)
- 2026-07-23 05:22 — staged 3.1.2 runtime acceptance passed with isolated settings: tray-enabled X hid the GUI while the collector process stayed alive; the singleton Open route restored the window; disabling close-to-tray made X terminate cleanly (agent: codex)
- 2026-07-23 05:23 — release CLI help and snapshot commands left the GUI process count unchanged at zero, while Native tests continued to prove `app.open` and `app.quit` dispatch; the TUI remains tray-free (agent: codex)
- 2026-07-23 05:36 — hosted Windows and both macOS Native GUI targets passed the first PR matrix, including the macOS status-item runtime seam; a shared Linux analysis-only libc declaration was corrected and locally revalidated before rerunning the matrix (agent: codex)
