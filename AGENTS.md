# AGENTS.md

This file provides guidance to Codex (Codex.ai/code) when working with code in this repository.

## Build & Test Commands

```bash
cargo build                    # Debug build
cargo build --release          # Release build (with LTO)
cargo run                      # Run TUI (interactive mode selection)
cargo run -- --user            # Launch directly into User Mode
cargo run -- --tech            # Launch directly into Technician Mode
cargo run -- update            # Run self-update action (preferred command form)
cargo run -- --update          # Run self-update action (legacy flag form)
cargo run -- --help            # Show help with keybindings and sections
cargo clippy                   # Lint
cargo test                     # Run tests (assert_cmd/predicates available for CLI integration tests)
```

The binary is named `sd300` (not `sd-300`). The crates.io package name is `SD300`; Cargo/crates.io install lookup is case-insensitive, so `cargo install sd300` resolves to the same package. The Rust library target is `sd_300`.

## Release Process (cargo-dist + crates.io)

The standard deploy path is a push to the repository default branch (`main`) with a new, unreleased `Cargo.toml` version. `.github/workflows/release.yml` is intentionally customized from cargo-dist output to match ND-300's probe-and-publish model; do not overwrite it with a generated cargo-dist workflow unless you preserve the main-branch deployment gate and crates.io publish job.

1. Bump version in `Cargo.toml`
2. Update `CHANGELOG.md` with new version entry
3. Update `README.md`, `CODEX_PROJECT.md`, `AGENTS.md`, and `CLAUDE.md` for user-visible release/install/update workflow changes
4. Run local verification: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --locked`, `cargo build --release --locked`, `cargo publish --dry-run --locked --allow-dirty`, cross-target `cargo check`, and `dist plan`
5. Commit and push to `main`
6. Wait for GitHub Actions to build successfully before treating the release as published

On `main`, the release workflow reads the package name/version, checks crates.io, GitHub Releases, and tags, then:
- skips deployment if the exact version is already fully published everywhere
- fails on partial-release states so a human can repair or bump forward
- runs cargo-dist artifact builds for all configured targets before hosting anything
- creates the `v{VERSION}` GitHub release and installer assets
- publishes the `SD300` crate only after the cargo-dist host job succeeds

Version tag pushes (`v*.*.*`) remain supported for explicit/manual releases, but the normal automation path is main-branch push. `CARGO_REGISTRY_TOKEN` must exist as a GitHub Actions secret; never commit registry tokens or publish from a local machine unless the user explicitly asks for an emergency manual publish after CI status has been checked.

cargo-dist builds for 6 targets (x86_64/aarch64 across Windows/macOS/Linux) and produces `SD300-*` archives plus shell, PowerShell, and MSI installers. `allow-dirty = ["ci"]` is set in `Cargo.toml` because the release workflow has deliberate deployment-gate customizations.

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
