# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Task management system

This repository uses the git-tracked `.tasks/` board for milestones, active work, verification, and cross-session handoff. At session start, read `.tasks/TASKS.md`, `.tasks/MILESTONES.md`, `.tasks/CLAUDE.md`, and every Active task's detail file. Keep each Active task's `## Status`, `## Verification`, and newest `## Activity` entry current as work progresses. Launch or repair the live board with `node .tasks/board-server.mjs ensure --open` and read its identity-bound port from `.tasks/.board-server.json`.

### Skill routing and current guidance

Use `/tasks-start` to initialize, repair, upgrade, relaunch, or resume the board.
`/tasks-create` is the preferred way to add a well-formed milestone, task, or proper
dashboard-visible subtask; `tasks-management` is the format and completion contract.
Use `/tasks-update` to upgrade and reconcile the existing board, sync/triage current
work, and refresh memory. `tasks-memory` governs that memory, `tasks-boards` governs
live-server identity, and `/tasks-remove` decommissions the system. As work changes,
keep `.tasks/TASKS.md` plus each Active task's `## Status` and `## Activity` current.

If the installed tasks plugin is missing or may be older than the board, first try the
harness-native plugin update. If that is unavailable, fails, or still leaves freshness
uncertain, use the GitHub skill/connector to read the relevant current `main` file under
`RealEmmettS/shaughv-tasks/skills/<skill-name>/SKILL.md` and use it as the latest
operating guidance: https://github.com/RealEmmettS/shaughv-tasks/tree/main/skills

## Changelog rule

This repository maintains two changelogs in parallel:

- `CHANGELOG.md` is the technical record. Preserve the project's existing release,
  file, command, metric, and qualification detail here.
- `HUMAN_CHANGELOG.md` is the plain-English companion. Every technical entry must have
  a corresponding explanation for a non-engineer: no version numbers, code references,
  or jargon, just what changed and why it matters.

Update both files in the same commit. Translate internal-only changes under **Behind the
scenes** rather than omitting them, and use the plain labels **Added**, **Improved**,
**Fixed**, **Removed**, **Security**, and **Behind the scenes** as applicable.

## Build & Test Commands

```bash
cargo build                    # Debug build
cargo build --release          # Release build (with LTO)
cargo run                      # Run TUI (interactive mode selection)
cargo run -- --user            # Launch directly into User Mode
cargo run -- --tech            # Launch directly into Technician Mode
cargo run -- gui               # Launch or focus the installed native GUI
cargo run -- update            # Run self-update action (preferred command form)
cargo run -- --update          # Run self-update action (legacy flag form)
cargo run -- install           # Deliberate preferred managed install
cargo run -- uninstall         # Uninstall through proven owner
cargo run -- snapshot --json   # Redacted noninteractive diagnostic snapshot
cargo run -- capabilities --json # Capability/provenance matrix
cargo run -- --help            # Show help with keybindings and sections
cargo clippy                   # Lint
cargo test                     # Run tests (assert_cmd/predicates available for CLI integration tests)
cd gui && npm ci               # Restore the exact Native SDK dependency graph
cd gui && npm run check        # Native SDK strict model/manifest check
cd gui && npm test             # Native SDK/Zig tests
scripts/build-native-gui.ps1 -Target windows-x86_64 # Release-style GUI build
```

The binary is named `sd300` (not `sd-300`). The crates.io package name is `tr300-tui`; use `cargo install tr300-tui` for Cargo installs. The Rust library target is `sd_300`.

## v3 CLI/TUI/GUI compatibility contract

- Bare `sd300` must continue to open the existing User/Technician chooser. Do
  not replace `App::run()`, move it onto the GUI runtime, auto-launch the GUI,
  or change established TUI commands, flags, keybindings, cadence, rendering,
  output contracts, exit codes, or terminal behavior as part of GUI work.
- `sd300 gui` is the only additive public launch command. It launches or focuses
  the installed app and reports a managed install/update repair instruction if
  the companion is absent.
- Managed/native install and update own CLI+GUI as one composite product but
  never launch the app. Proven-owner uninstall removes both frontends and their
  owned integrations/data without deleting ambiguous paths or user exports.
- Existing Cargo-owned v2 users intentionally update twice: Cargo installs the
  v3 CLI first, then the second same-version update performs the transactional
  managed CLI+GUI takeover. This is the sole ownership exception.
- GUI/TUI feature parity is a release invariant. New collectors, fields,
  warning semantics, observation states, capabilities, provenance, redaction,
  and configurable features must be wired to both applicable frontends in the
  same change, or explicitly documented as platform/frontend unavailable.
- Keep settings namespaces separate. Only deliberately shared persistent
  settings belong under `shared`; window, navigation, GUI mode/unit, chart,
  tray, startup, close, and motion choices belong under `gui` and must never
  change TUI session defaults.

## Release Process (cargo-dist + crates.io)

The standard deploy path is a push to the repository default branch (`main`) with a new, unreleased `Cargo.toml` version. `.github/workflows/release.yml` is intentionally customized from cargo-dist output; do not overwrite it with a generated workflow unless you preserve the main-branch deployment gate, unpublished qualification draft, native matrices, and final crates.io/latest publish gate.

For v3 GUI work, keep implementation and qualification on a `codex/` feature
branch first. Build and exercise the composite Windows MSI before merging to
`main`; a successful compile is not installer acceptance.

1. Bump version in `Cargo.toml`
2. Update `CHANGELOG.md` with new version entry
3. Update `README.md`, `gui/README.md`, `CODEX_PROJECT.md`, `AGENTS.md`, and `CLAUDE.md` for user-visible release/install/update workflow changes
4. Reconcile the root crate, GUI engine, npm package/lock, Zig manifests, staged templates, and package metadata to one product version; run `npm --prefix gui run check:product-version`
5. Run local verification: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --locked`, `cargo build --release --locked`, `cargo publish --dry-run --locked --allow-dirty`, cross-target `cargo check`, `dist plan`, `npm --prefix gui ci`, `npm --prefix gui run check`, and `npm --prefix gui test`
6. Build only through the target-pinning GUI wrappers and prove a clean-cache build, warmed/offline dependency build, GUI self-test, distribution-lock check, and developer-path/debug-symbol leakage scan
7. Qualify an unpublished draft, including v2.0.6 compatibility, all owner/update/repair/rollback/uninstall routes, all six CLI+GUI targets, and the complete performance matrix
8. Generate checksums and SPDX SBOMs, attest every release subject, and publish the crate/release only after every candidate gate succeeds
9. Verify exact public bytes, attestations, stable routers, install/update/repair/uninstall, and application discovery before calling the release complete

On `main`, the release workflow reads the package name/version, checks crates.io, GitHub Releases, and tags, then:
- skips deployment if the exact version is already fully published everywhere
- repairs a crates.io-published/GitHub-release-missing state by rebuilding artifacts and finishing release hosting
- fails other partial-release states so a human can repair or bump forward
- runs cargo-dist artifact builds for all configured targets before hosting anything
- creates an unpublished `v{VERSION}` draft after all cargo-dist artifacts build
- renders stable managed wrappers, internal exact-tag cargo-dist installers, and immutable-client compatibility routers with SHA-256 sidecars
- builds and exercises Global/Corporate MSI and EXE installers on Windows, including synthetic-prior real self-updates, complete CLI uninstalls, and deliberate fresh takeovers into the managed PowerShell channel through all four native lanes
- builds a signed/notarized universal PKG and exercises managed-shell uninstall plus synthetic-prior same-PKG update/uninstall on native Intel and Apple Silicon runners
- builds the GUI and bundle-relative Rust engine for Windows x86_64, macOS
  x86_64/ARM64, Linux GNU x86_64/ARM64, and Linux musl x86_64; x86_64 is the
  Intel/AMD target, and Windows ARM64 is not part of this six-target contract
- qualifies the complete candidate lifecycle before publishing `tr300-tui` or
  changing the draft to `latest`; post-public checks verify the same public bytes
- creates SHA-256 sidecars, an SPDX SBOM, and GitHub attestations for the release
  subjects; `gh attestation verify <asset> -R QubeTX/qube-system-diagnostics`
  is the documented provenance check, not a substitute for platform code signing

Version tag pushes (`v*.*.*`) remain supported for explicit/manual releases, but the normal automation path is main-branch push. `CARGO_REGISTRY_TOKEN` must exist as a GitHub Actions secret; never commit registry tokens or publish from a local machine unless the user explicitly asks for an emergency manual publish after CI status has been checked.

The package was moved to `tr300-tui` so the project can publish while keeping the installed command and product identity as `sd300` / SD-300. After release, verify `cargo install tr300-tui --version {VERSION}` installs `sd300` and that the GitHub Release assets are present.

cargo-dist builds for 6 targets (x86_64/aarch64 across Windows/macOS/Linux) and produces `tr300-tui-*` archives containing the `sd300` binary. Fresh installs advertise stable `sd300-cli-installer.ps1` / `sd300-cli-installer.sh`; native options are stable Global/Corporate MSI/EXE names and `sd300-macos-universal.pkg`. `tr300-tui-installer.*` and uppercase `SD300-installer.*` remain compatibility routers for immutable 1.4.x clients. The `-cli-` segment prevents GitHub's case-equivalent asset collision with the uppercase bridge. Updater internals resolve the latest tag once and use exact-tag URLs plus SHA-256 sidecars. `allow-dirty = ["ci", "msi"]` is set because CI and installer naming are deliberately customized.

The GUI dependency graph is distribution-locked by `gui/toolchain-lock.json`,
`gui/package-lock.json`, and `gui/build.zig.zon`: `@native-sdk/cli` is exactly
0.5.4 and Zig is exactly 0.16.0, with immutable URLs, npm integrity/content
hashes, per-host Zig SHA-256 values, and the reviewed renderer patch. Do not
introduce `.path` dependencies, profile/global npm paths, local SDK checkouts,
unpinned branches, or a requirement for customer-side compilation.

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

The GUI loads the Rust library as `sd300_engine.dll`,
`libsd300_engine.dylib`, or `libsd300_engine.so` from an absolute
bundle-relative path. It owns a separate non-cloneable `SystemSnapshot` and
Tokio runtime on a dedicated engine thread, publishes bounded/versioned topic
projections, and never shares mutable state or scheduling with the TUI process.

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
- **`collectors/thermals.rs`** — Cross-platform component sensors plus Windows Libre/Open Hardware Monitor WMI bridges, guarded read-only Dell AWCC enumeration, ACPI fallback, and independent GPU-temperature merging. Dell control methods must never be called by the collector.
- **`ui/common.rs`** — Color palette, `content_block()`/`sub_block()` panel helpers, `gauge_bar()`, `format_bytes()`, sparkline bar sets. All UI constants (colors, sparkline colors) are defined here.
- **`ui/sections/`** — One file per section (9 sections), each rendering User and Tech mode independently.
- **`types.rs`** — Core enums: `DiagnosticMode`, `Section` (1-9), `HealthStatus`, `ProcessSortKey`, `TempUnit`, `DeviceCategory` (9 variants), `DriverScanStatus` (4 variants).
- **`history.rs`** — `HistoryBuffer`: fixed-capacity ring buffer (VecDeque) for sparkline data (60 samples default).
- **`gui-engine/`** — `cdylib` C ABI over the shared Rust collectors; no Rust
  panic, allocation, reference, or borrowed buffer may cross the ABI.
- **`gui/src/main.zig`** — Native GUI `Model`, tagged `Msg`, update effects,
  engine bridge, settings, and bounded view histories.
- **`gui/src/app.native`** — Declarative Native SDK view hierarchy and bindings.
- **`gui/src/fonts/`** — embedded Makira and IBM Plex Mono font binaries;
  license notices and retained evidence live under `gui/assets/fonts/`.

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
- **One-second data, bounded presentation** — preserve live collector cadence
  while consuming sequence changes/latest-only projections. A visible GUI must
  present fast-topic samples at least once per second after renderer
  optimization; hidden/tray mode may coalesce to its required summaries. Never
  create an unbounded renderer queue or hide a collector regression by lowering
  data fidelity.
- **Tray/startup lifecycle** — tray and launch-at-login are independent and
  default off. Windows/macOS close-to-tray only when enabled. Linux has no tray
  under Native SDK 0.5.4, closes normally, and must never autostart hidden.
- **Performance is release-blocking** — qualify 15-minute foreground,
  30-minute hidden, and two-hour soak runs. Budgets are <=2% of one logical core
  foreground, <=1% hidden, <=150 MiB working set/RSS, <=300 MiB private
  memory/commit, <=16.7 ms frame p95, and <=50 ms input p95 outside scans, with
  no ordinary refresh stall over 100 ms or unbounded growth.

### Visual systems

The Ratatui TUI retains its warm earth palette and existing helpers unchanged.
The native GUI uses the Warm Carbon identity: near-black/charcoal surfaces,
controlled orange/amber status energy, restrained gradients, and a subtle
opacity-faded grid rather than generic purple “AI” styling. Makira is primary
for body copy, headings, and major numerals; IBM Plex Mono is secondary for
technical labels and compact values. Do not silently substitute or redistribute
fonts without preserving the applicable embedding-license evidence.
