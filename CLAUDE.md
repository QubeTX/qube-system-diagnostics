# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Task management system

This repository uses the git-tracked `.tasks/` board for milestones, active work, verification, and cross-session handoff. At session start, read `.tasks/TASKS.md`, `.tasks/MILESTONES.md`, `.tasks/CLAUDE.md`, and every Active task's detail file. Keep each Active task's `## Status`, `## Verification`, and newest `## Activity` entry current as work progresses. Launch or repair the live board with `node .tasks/board-server.mjs ensure --open` and read its identity-bound port from `.tasks/.board-server.json`.

## Build & Test Commands

```bash
cargo build                    # Debug build
cargo build --release          # Release build (with LTO)
cargo run                      # Run TUI (interactive mode selection)
cargo run -- --user            # Launch directly into User Mode
cargo run -- --tech            # Launch directly into Technician Mode
cargo run -- update            # Run self-update action (preferred command form)
cargo run -- --update          # Run self-update action (legacy flag form)
cargo run -- install           # Deliberate preferred managed install
cargo run -- uninstall         # Uninstall through proven owner
cargo run -- snapshot --json   # Redacted noninteractive diagnostic snapshot
cargo run -- capabilities --json # Capability/provenance matrix
cargo run -- --help            # Show help with keybindings and sections
cargo clippy                   # Lint
cargo test                     # Run tests (assert_cmd/predicates available for CLI integration tests)
```

The binary is named `sd300` (not `sd-300`). The crates.io package name is `tr300-tui`; use `cargo install tr300-tui` for Cargo installs. The Rust library target is `sd_300`.

## Release Process (cargo-dist + crates.io)

The standard deploy path is a push to the repository default branch (`main`) with a new, unreleased `Cargo.toml` version. `.github/workflows/release.yml` is intentionally customized from cargo-dist output; do not overwrite it with a generated workflow unless you preserve the main-branch deployment gate, unpublished qualification draft, native matrices, and final crates.io/latest publish gate.

1. Bump version in `Cargo.toml`
2. Update `CHANGELOG.md` with new version entry
3. Update `README.md`, `CODEX_PROJECT.md`, `AGENTS.md`, and `CLAUDE.md` for user-visible release/install/update workflow changes
4. Run local verification: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --locked`, `cargo build --release --locked`, `cargo publish --dry-run --locked --allow-dirty`, cross-target `cargo check`, and `dist plan`
5. Commit and push to `main`
6. Wait for the base draft, Windows native matrix, Intel/Apple Silicon PKG matrix, and final qualification workflow before treating the release as published

On `main`, the release workflow reads the package name/version, checks crates.io, GitHub Releases, and tags, then:
- skips deployment if the exact version is already fully published everywhere
- repairs a crates.io-published/GitHub-release-missing state by rebuilding artifacts and finishing release hosting
- fails other partial-release states so a human can repair or bump forward
- runs cargo-dist artifact builds for all configured targets before hosting anything
- creates an unpublished `v{VERSION}` draft after all cargo-dist artifacts build
- renders stable managed wrappers, internal exact-tag cargo-dist installers, and immutable-client compatibility routers with SHA-256 sidecars
- builds and exercises Global/Corporate MSI and EXE installers on Windows, including synthetic-prior real self-updates through all four preserved channels
- builds a signed/notarized universal PKG and exercises a synthetic-prior same-PKG update on native Intel and Apple Silicon runners
- publishes the `tr300-tui` crate only after the complete native asset/checksum/test matrix passes
- publishes the draft as `latest` immediately after the qualified crate step, then proves public Linux managed-shell and Cargo updates plus fresh install/uninstall

Version tag pushes (`v*.*.*`) remain supported for explicit/manual releases, but the normal automation path is main-branch push. `CARGO_REGISTRY_TOKEN` must exist as a GitHub Actions secret; never commit registry tokens or publish from a local machine unless the user explicitly asks for an emergency manual publish after CI status has been checked.

The package was moved to `tr300-tui` so the project can publish while keeping the installed command and product identity as `sd300` / SD-300. After release, verify `cargo install tr300-tui --version {VERSION}` installs `sd300` and that the GitHub Release assets are present.

cargo-dist builds for 6 targets (x86_64/aarch64 across Windows/macOS/Linux) and produces `tr300-tui-*` archives containing the `sd300` binary. Fresh installs advertise stable `sd300-cli-installer.ps1` / `sd300-cli-installer.sh`; native options are stable Global/Corporate MSI/EXE names and `sd300-macos-universal.pkg`. `tr300-tui-installer.*` and uppercase `SD300-installer.*` remain compatibility routers for immutable 1.4.x clients. The `-cli-` segment prevents GitHub's case-equivalent asset collision with the uppercase bridge. Updater internals resolve the latest tag once and use exact-tag URLs plus SHA-256 sidecars. `allow-dirty = ["ci", "msi"]` is set because CI and installer naming are deliberately customized.

## Architecture

### Data Flow

```
main.rs → App::run() event loop → tokio::select! {
    fast_tick (1s):   refresh_fast()   → CPU, memory, network, processes
    slow_tick (5s):   refresh_slow()   → disk, GPU, thermals
    medium_tick (3s): refresh_connections() → active sockets
    diag_tick (15s):  spawn_blocking(connectivity) → gateway, DNS, internet
    health_tick (60s): refresh_disk_health() → SMART data
    event_stream:     handle_event() → keyboard input
}
```

All system data lives in `App.snapshot: SystemSnapshot`, which holds a non-Clone `sysinfo::System` internally. Collectors read from this shared System instance.

### Rendering Pipeline

```
ui::render(frame, app)
  → header_bar::render()           # 2-line title bar
  → sections::render(section)      # Dispatches to active section
      → {section}::render(mode)    # Each section has render_user() and render_tech()
  → bottom_bar::render()           # Tab navigation
  → help_overlay::render()         # If show_help is true (rendered on top)
```

Every section module has a `render(frame, app, area, mode)` function that branches into `render_user()` (plain language) and `render_tech()` (raw data). Minimum terminal size is 80x24.

### Module Layout

- **`app.rs`** — App state, event loop, 5 refresh intervals, async driver scan polling
- **`collectors/`** — Each collector returns a typed data struct. `SystemSnapshot` owns all of them and has refresh methods that delegate to individual collectors.
- **`collectors/drivers/platform/`** — Platform-dispatched driver scanning: Windows uses Setup API (`SetupDi*`), Linux uses sysfs, macOS uses IOKit. Selected at compile time via `#[cfg(target_os)]`.
- **`ui/common.rs`** — Color palette, `content_block()`/`sub_block()` panel helpers, `gauge_bar()`, `format_bytes()`, sparkline bar sets. All UI constants (colors, sparkline colors) are defined here.
- **`ui/sections/`** — One file per section (9 sections), each rendering User and Tech mode independently.
- **`types.rs`** — Core enums: `DiagnosticMode`, `Section` (1-9), `HealthStatus`, `ProcessSortKey`, `TempUnit`, `DeviceCategory` (9 variants), `DriverScanStatus` (4 variants).
- **`history.rs`** — `HistoryBuffer`: fixed-capacity ring buffer (VecDeque) for sparkline data (60 samples default).

### Platform Patterns

Windows-only deps (`wmi`, `serde`, `winapi`, `windows` crate) are gated under `[target.'cfg(windows)'.dependencies]`. Unix-only deps (`libc`, `nix`) under `[target.'cfg(unix)'.dependencies]`.

In source code, use `#[cfg(target_os = "windows")]` / `#[cfg(target_os = "linux")]` / `#[cfg(target_os = "macos")]` for platform-specific blocks. The driver scanning module (`collectors/drivers/platform/mod.rs`) is the primary example of this dispatch pattern.

### Key Constraints

- **`SystemSnapshot` is not Clone** — it owns `sysinfo::System` which has no Clone impl. Don't try to derive Clone on types containing it.
- **Async driver scanning** — Driver collection is slow (Setup API/sysfs enumeration). It runs via `tokio::task::spawn_blocking` and the result is polled via `JoinHandle::is_finished()` before each draw cycle. Never call `drivers::collect()` on the main thread.
- **Warning deduplication** — Warnings are cleared per-source before re-collecting: `warnings.retain(|w| w.source != "SourceName")`. Always follow this pattern when adding new warning sources.
- **UI helpers** — Use `content_block(title)` for outer section panels and `sub_block(title)` for nested subsections. Use the existing `COLOR_*` and `SPARK_*` constants from `common.rs` — don't hardcode RGB values.
- **Sparkline rendering** — Windows uses `THREE_LEVELS` bar set, Unix uses `NINE_LEVELS`. The `sparkline_bar_set()` function handles this automatically.
- **build.rs** — Generates a man page (`sd300.1`) via `clap_mangen` at build time. It includes `src/cli.rs` via `#[path]` attribute, so the Cli struct must remain compatible with both the main binary and the build script.

### Color Palette

Warm earth tones throughout — sage green (good), warm amber (warning), terracotta red (critical), warm gold (accent), warm white (text), dark warm gray (borders). Named sparkline colors: `SPARK_CPU`, `SPARK_MEMORY`, `SPARK_SWAP`, `SPARK_NET_DOWN`, `SPARK_NET_UP`, `SPARK_GPU`, `SPARK_TEMP`.
