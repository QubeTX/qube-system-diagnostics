# SD-300 desktop process and lifecycle model

Date: 2026-07-20
Status: accepted implementation contract for the v3 qualification branch

## Product boundary

SD-300 has two first-class frontends over the same collector code, not one frontend
that launches the other:

| Surface | Process ownership | Startup | Close/exit contract |
|---|---|---|---|
| CLI/TUI | `sd300` owns the existing Ratatui loop and its `SystemSnapshot` | A terminal runs `sd300`; bare invocation still opens the existing chooser | Existing terminal keys, signals, output, and exit behavior |
| Desktop app | `sd300-gui` owns one Native SDK event loop and one isolated Rust-engine runtime | Start/Search, Applications, desktop menu, or additive `sd300 gui` | Windows/macOS close hides to the app's tray; tray Open restores; tray Quit shuts down gracefully |
| Linux desktop app | `sd300-gui` owns one Native SDK event loop and one isolated Rust-engine runtime | Desktop menu or additive `sd300 gui` | Close exits; no tray or hidden-only process while Native SDK lacks Linux tray support |
| Lifecycle owner | Existing MSI/EXE/PKG/managed installer and updater routes | Explicit install, update, repair, or uninstall | Stops the GUI, commits or rolls back CLI+GUI together, never becomes a monitor process |

The GUI never starts a terminal in order to collect data. The TUI never embeds the
Native SDK runtime. Opening one frontend does not require the other frontend to be
running.

## One desktop app and tray process

On Windows and macOS the window and tray are two projections of the same model in the
same `sd300-gui` process. Native SDK 0.5.4 supplies the required lifecycle primitives:

1. The startup window declares `close_policy = "hide"`.
2. The status item is installed on the first presented frame.
3. Closing hides the existing window without destroying it.
4. `app.open` maps to `Effects.showWindow("main")` and restores/activates it.
5. `app.quit` maps to `Effects.quitApp()` and follows the normal graceful shutdown path.
6. On Windows, the SDK downgrades close to a real close if the tray failed to install,
   so a user cannot be left with an unreachable background process.

Linux has a separate manifest with `close_policy = "quit"` and no `tray` capability.
This is a build-time invariant, not a UI convention. A Linux package therefore cannot
accidentally hide its only window.

## Collector ownership and performance

Each frontend owns a different, non-cloneable `SystemSnapshot`:

- The TUI keeps its existing `App::run()` scheduling and refresh methods unchanged.
- The GUI loads `sd300_engine` from its bundle and gives it a dedicated Rust thread and
  Tokio runtime.
- The GUI receives bounded, serialized, latest-only projections. A slow renderer can
  skip sequence numbers but cannot accumulate a queue.
- The initial Overview profile refreshes only CPU and memory. It does not poll network
  connections, disks, GPU, thermals, connectivity diagnostics, health, or drivers until
  a future visible view explicitly selects a profile that needs them.
- Hidden/tray mode currently costs no more collection work than Overview. A later profile
  transition may narrow the summary further, but must keep tray values honest and fresh.

The renderer is sequence-driven and uses a repeating sample timer, not a permanent
decorative animation. Tables and histories must remain bounded.

## Windows subprocess contract

The terminal flashing incident exposed an important distinction:

- Release compilation legitimately starts many compiler/linker subprocesses and can be
  CPU intensive, but it is a developer/CI operation and not an installed-app behavior.
- Runtime collectors may invoke read-only OS utilities such as `netstat`, `route`,
  `ping`, or vendor probes. When a Windows GUI-subsystem process spawns them normally,
  Windows may create a visible console window for each child.

All noninteractive collector commands now flow through one helper that nulls stdin,
captures stdout/stderr, and sets Windows `CREATE_NO_WINDOW`. A regression test starts a
probe through that exact helper and asserts that `GetConsoleWindow()` returns null.
Installer elevation, explicit interactive shell operations, and updater handoff remain
outside this helper and retain their existing behavior.

## Distribution and activation rules

- Customers receive centrally built, baseline-CPU, target-specific bytes. Installation
  never invokes Zig, Rust, npm, or a shell compiler on an end-user device.
- The Native SDK CLI, npm tarball/integrity, SDK commit, Zig version/hash, and Zig package
  hash are updated as one reviewed lock.
- The GUI engine is loaded only from an absolute bundle-relative path and rejects ABI,
  schema, product, target, or version mismatch before collection starts.
- Normal launch is single-instance/focus behavior once the activation IPC is completed.
  A second launch must focus the existing app rather than create a second collector.
- Launch-at-login is a GUI preference and remains off by default. Windows/macOS may start
  hidden only when tray is enabled. Linux always starts visibly.
- Install and update stage the app but do not launch it. Uninstall stops it and removes
  the tray/autostart registration before removing owned files.

## Verification sequence

No interactive run occurs until strict source/model checks, unit tests, optimized build,
dependency lock, path-leak scan, and Windows no-console tests are green. Live qualification
then proves, in order: nonblank frame, live values, one process, close-to-hide, tray Open,
tray Quit, no console flashes, foreground/hidden resource use, and clean shutdown. The
same exact bytes are then installed and removed through the Corporate MSI candidate.

## Sources

- [Native SDK application lifecycle and CLI](https://native-sdk.dev/cli)
- [Native SDK packaging](https://native-sdk.dev/packaging)
- [Native SDK security model](https://native-sdk.dev/security)
- [Microsoft process creation flags](https://learn.microsoft.com/en-us/windows/win32/procthread/process-creation-flags)
