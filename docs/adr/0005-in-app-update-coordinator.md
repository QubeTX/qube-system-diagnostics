# ADR 0005 — In-app updates spawn the CLI as a detached coordinator; the GUI never mutates the installation

Status: Accepted (2026-07-22). Shipped as v3.1.0 (task #giu on the board).

## Context

The operator wanted the nontechnical update experience: click "Update now" in
the desktop app or its tray menu, have the complete CLI+GUI product update
through its proven owner, and have the app reopen when the update finishes.

Two prior constraints framed the design. First, the v3.0.0 contract said
managed/native install and update never launch the app — GUI lifecycle
surfaces were status-only so installer ownership, elevation, rollback, and the
JSON contracts stayed authoritative in the CLI. Second, the reference
investigation in the Goose repository concluded that a tray/GUI process must
never update itself: the running image cannot safely replace its own bytes,
and an in-UI updater inevitably grows into a second, divergent lifecycle
implementation.

## Decision

One new path, five hops, each with a single responsibility:

1. **GUI intent** (`gui/src/main.zig`). A Settings-page "Update now" row and a
   tray "Update SD-300" item both send the typed message `Msg.update_now`
   (tray routes through the command string `app.update` in `onCommand`). The
   handler `requestUpdateNow` only calls the engine and sets the status line —
   the UI process performs no lifecycle work.
2. **Engine ABI** (`gui-engine/src/lib.rs`). `sd300_engine_request_update`
   is a `catch_unwind`-guarded `extern "C"` entry like every other. Its return
   status is authoritative; the caller-owned message buffer is best-effort
   context (resolved CLI path on success, failure reason otherwise) and a
   too-small buffer must never turn a spawned coordinator into an apparent
   failure. The symbol is additive — no ABI version bump, because the GUI's
   hard `try library.lookup` plus the self-test's exact product-version
   equality already pin engine/app pairs shipped in one bundle.
3. **Coordinator spawn** (`src/gui.rs::spawn_update_coordinator`). The engine
   (running inside the GUI process) resolves the installed CLI from proven
   absolute locations only, in order: the composite root's `bin/` sibling of
   the app directory (`<root>/app/sd300-gui` ↔ `<root>/bin/sd300`), the flat
   shared `bin/` layout (Linux managed installs), the managed receipt's
   recorded binary (`managed_cli_binary`), and the fixed macOS PKG path
   `/usr/local/bin/sd300`. **Never a PATH lookup** — that is the command/path
   injection surface the design forbids. The child runs
   `update --json --relaunch-gui`, detached: `CREATE_NO_WINDOW` on Windows,
   `process_group(0)` on Unix, stdin null, stdout/stderr to
   `update-launch.log` beside the GUI settings (an unwritable log falls back
   to null rather than blocking the update).
4. **The unchanged transaction** (`src/update.rs::run`). The CLI performs its
   normal owner-preserving update exactly as from a terminal, including asking
   the GUI to exit through the authenticated quit endpoint before mutating
   bytes. The coordinator outliving its GUI parent is what makes this safe.
5. **Gated relaunch** (`src/update.rs::run_with_relaunch`). The pure gate
   `should_relaunch_gui(requested, exit_code)` requires BOTH the hidden
   `--relaunch-gui` flag AND exit code 0. Only then does the CLI call
   `gui::launch()`. Relaunch problems report on stderr only; the update's
   single-JSON-object stdout and its exit code are byte-identical to a
   terminal run (proven by the v2.0.6 compat goldens).

**Idempotence.** An already-current product exits 0 without ever stopping the
GUI; the relaunch then hits the single-instance claim, which notifies and
focuses the running app instead of duplicating it. A real update stopped the
GUI mid-transaction, so the relaunch starts the one new verified instance.

**Failure honesty.** A failed transaction exits nonzero → no relaunch → the
proven prior product remains installed and the log holds the evidence. A
missing/unresolvable CLI is reported on the GUI status line with the terminal
fallback instruction. Two rapid clicks spawn two coordinators — deliberately
the same exposure as two terminals running `sd300 update`, protected by the
transaction's existing conflict handling, not new UI state.

## Rejected alternatives

- **Updater logic in the GUI process** — a second lifecycle implementation
  that would drift from the CLI's ownership/elevation/rollback rules, and a
  process trying to replace its own running image.
- **GUI-driven relaunch** — the GUI is dead mid-update; only the surviving
  coordinator knows the outcome.
- **PATH-based CLI resolution** — injection surface; forbidden.
- **Public `--relaunch-gui`** — it is a coordinator implementation detail;
  hiding it keeps `--help` byte-identical for immutable compat fixtures.
- **ABI version bump for the new symbol** — additive lookup + product-version
  equality already fail a mismatched pair loudly at engine init.
- **Blocking the UI or polling for update completion** — the app is about to
  be exited by the transaction; a status line plus the log is the honest UI.

## Consequences

- The contract exception is recorded in CLAUDE.md, AGENTS.md,
  CODEX_PROJECT.md, README.md, and gui/README.md: only the hidden flag plus a
  successful transaction may launch the app; installs, terminal updates, and
  failures never do.
- Uninstall remains CLI-only by product decision (the one function the two
  frontends do not share).
- Tray behavior is verified by programmatic `onCommand`/Msg dispatch plus a
  MANUAL OPERATOR TEST of the physical click — Computer Use cannot see the
  Windows tray (post-mortem rule; do not loop on it).
- The version-consistency checker now treats literal `v<semver>` markup labels
  as optional: visible versions bind to `{productVersionLabel}`, which derives
  from the checked `gui/src/engine.zig` constant, so labels cannot go stale.
- `gui-engine` carries 21 pre-existing `not_unsafe_ptr_arg_deref` clippy hits
  across its C-ABI surface (root-crate clippy is the CI gate). The new entry
  deliberately matches the established pattern; a crate-wide lint decision is
  #hrd backlog work, not something to bundle into a release.
