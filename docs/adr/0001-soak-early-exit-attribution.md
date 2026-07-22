# ADR 0001 — Soak early-exit attribution: operator window close, not a product defect

Date: 2026-07-22
Status: Accepted
Related: `.tasks/tasks/gux.md`, `.tasks/tasks/sok.md`, `scripts/measure-native-gui-performance.ps1`, `gui/src/main.zig`, `gui/src/platform/window_visibility.zig`

## Context

The planned two-hour Processes soak of the exact final v3 Windows app (SHA-256
`d83de90b…7acaa2`, PID 4260) started 2026-07-21 23:38 and ended at 00:03:52 with
exit code 0 after ~25 minutes. The harness threw at
`scripts/measure-native-gui-performance.ps1:257` and left an empty JSON result.
No crash event existed. The prior handoff treated the graceful-exit source as an
open release blocker and instructed: do not rerun blindly.

## Evidence

1. **The harness cannot cause an early exit.** Its only interaction with the
   app's quit endpoint sits in the `finally` block behind
   `if ($null -ne $process -and -not $process.HasExited)` — unreachable when the
   process already exited. The measurement loop only observes `HasExited` and
   throws (line 257). The harness launches exactly one process and never spawns
   a second instance.
2. **No scheduled or background quit source exists.** Repository-wide search
   found no scheduled task, poller, or auto-update path. Every caller of the
   authenticated quit endpoint (`Local\SD300.Gui.Quit.v1`) is an explicit CLI
   subcommand (`update`/`install`/`uninstall`/installer-only helpers). The app
   has no watchdog or idle timer. A second `sd300 gui` launch focuses — never
   quits — the first instance.
3. **The close mechanism produces exactly the observed signature.** On Windows
   the window close policy is compile-time `.hide`
   (`gui/src/main.zig:1406`). ANY close signal (title-bar X, Alt+F4, automation
   close) hides the window; within one second the refresh tick evaluates
   `shouldQuitForHiddenWindow(tray_enabled=false, policy_hidden=true)`
   (`main.zig:388-397,568-574`) and quits gracefully with code 0. The harness
   writes `tray_enabled=false` for every foreground run.
4. **Environment.** Windows Application log shows a Honk300 v1.3.4 MSI physical
   acceptance ran 23:56:44–48 (concurrent Goose-project work on the same
   machine); the operator was physically present around midnight and confirmed
   probably opening/closing the SD-300 window at that time.

## Decision

Attribute the early clean exit to an operator window close. It is not a product
defect; no code change is required for the exit itself. The invalidated
pre-release soak was subsequently waived by the operator to post-release task
`#sok` (release-focus direction, 2026-07-22).

## Consequences

- Unattended long-run measurements MUST quarantine the machine: no interactive
  use, no concurrent installer/acceptance work, no Computer Use / UI Automation
  observers (previously proven to contaminate CPU readings).
- Anyone rerunning the soak should remember: with tray off, closing the window
  IS exiting the app within one second — by design.
- Optional hardening (not required now): bounded close-source logging in the app
  to make future exits self-attributing.
