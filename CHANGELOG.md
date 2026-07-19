# Changelog

All notable changes to SD-300 will be documented in this file.

## [2.0.3] - 2026-07-19

### Fixed

- Added a rollback-capable Windows native-uninstall live-image handoff so `sd300 uninstall` can return its final result instead of being terminated by MSI or EXE Restart Manager while removing the running binary.
- Removed empty SD-300 receipt directories after managed shell, managed PowerShell, and macOS PKG uninstall while preserving shared Cargo/Rust tooling and non-empty shared directories.
- Changed Windows native uninstall to resolve `msiexec.exe` from the trusted Windows system directory rather than executable-search `PATH`.
- Expanded release qualification so the CLI itself must completely remove all four Windows MSI/EXE channels, macOS managed shell and PKG channels, and Linux managed shell/Cargo channels, including registrations, markers, owned PATH entries, receipts, and payload roots.

## [2.0.2] - 2026-07-19

### Fixed

- Normalized cargo-dist's generated managed-install receipt identity and path from the historical package name `tr300-tui` to the product identity `sd300`, so production shell and PowerShell installs can be proven and updated through the managed channel.
- Added release-assembly assertions for the exact managed receipt identity, directory, and filename in both generated installer formats.

## [2.0.1] - 2026-07-19

### Fixed

- Drained child-process stdout and stderr concurrently so updater release checks cannot deadlock when GitHub's latest-release response exceeds an operating-system pipe buffer.
- Added repeatable post-public artifact and lifecycle qualification without republishing an already-public crate or release.

## [2.0.0] - 2026-07-18

### Added

- Added `sd300 install`, `sd300 uninstall`, `sd300 snapshot`, and `sd300 capabilities` with exactly-one-object JSON lifecycle output and privacy-redacted noninteractive diagnostics.
- Added stable `recovery_url` and `requires_user_action` fields to every JSON install, update, and uninstall result for cross-product automation parity.
- Added proven install-channel detection and same-channel updates for managed PowerShell/shell, Cargo, Global/Corporate MSI, Global/Corporate EXE, and signed universal macOS PKG ownership.
- Added stable, versionless public wrappers and native artifact names with SHA-256 sidecars, plus immutable 1.4.x compatibility routers that preserve provable native ownership across the v2 transition.
- Added authoritative fresh-install takeover with strict, bounded migration cleanup, rollback, downgrade-capable MSI packages, and opposite-scope Windows refusal before mutation.
- Added separate Global and Corporate Windows MSI/EXE packages, a direct Developer ID signed/notarized universal macOS PKG, and draft release qualification across Windows, Apple Silicon, Intel Mac, and Linux before publication as `latest`.
- Gated crates.io publication behind the same complete native asset and test matrix so v2 is not publicly installable through Cargo before Windows and macOS qualification succeeds.
- Changed raw Cargo ownership detection to require Cargo's structured install manifest, exact package/binary ownership, and matching version instead of trusting the `.cargo/bin` path alone.
- Required exact cargo-dist receipt fields in both updater detection and takeover cleanup; recursively nested lookalike keys no longer count as ownership proof.
- Added a Windows live-image handoff with an elevated, same-channel Global worker, verified rollback, and bounded detached cleanup so MSI/EXE Restart Manager behavior cannot kill the updater's final result or strand the prior executable.
- Resolve overlapping exact Cargo/managed ownership by the newer metadata record and fail closed on equal timestamps instead of guessing.
- Added candidate-to-candidate hosted version-transition gates for managed PowerShell, four Windows native channels, direct PKG on Intel/Apple Silicon, and Linux managed shell/Cargo ownership.
- Managed CLI wrappers now SHA-256 verify their exact-tag cargo-dist installer payload before execution.
- Added Windows memory-module, multi-GPU, display, physical-disk health/reliability, battery, hardware identity, native adapter/link-speed, and SetupAPI/WMI driver-status diagnostics.
- Added typed observation provenance so unavailable, unsupported, permission-denied, contradictory, and error states are never presented as fabricated zero telemetry.

- Added an exhaustive, privacy-sanitized macOS hardware-monitor capability and implementation report based primarily on live testing of a `Mac14,7` M2 MacBook Pro running macOS 26.3.1.
- Documented locally proven unprivileged access to 38 IOHID temperature services, read-only AppleSMC fan telemetry, IOReport energy/frequency residency, Metal/AGX data, battery/adapter internals, APFS/NVMe status, block-I/O counters, Wi-Fi radio state, displays, and device inventories.
- Added implementation-ready Rust guidance covering module boundaries, target dependencies, FFI ownership, exact private-interface call sequences and SMC ABI layout, collector cadence, availability/provenance types, redaction, fixtures, CI, and later Mac qualification.
- Documented the macOS 26 unprivileged `diskutil` NVMe SMART-detail dictionary, including spare/life-used/temperature/lifecycle/error fields, checked conversion rules, and its explicit non-guaranteed cross-version status.
- Added sanitized real request/response examples for `system_profiler`, `diskutil`, I/O Registry, IOHID, AppleSMC, and IOReport, with exact command envelopes and typed parser schemas but no stable machine, account, network, or device identifiers.
- Added a critical-thinking inquiry canvas that separates exact-host observations, public contracts, private/model-specific behavior, and unverified fleet assumptions.

### Changed

- Made `irm .../sd300-cli-installer.ps1 | iex` the recommended Windows install and `curl .../sd300-cli-installer.sh | sh` the recommended macOS/Linux install; native installers remain first-class options.
- Made a deliberate fresh official install the authoritative latest user intent regardless of installed version, while raw `cargo install tr300-tui` remains an advanced unmanaged option because Cargo provides no post-install ownership hook.

- Corrected project documentation so macOS is described as a supported baseline with known telemetry gaps instead of claiming comprehensive IOKit-based/full monitoring.
- Recorded Vercel Labs Native as an optional pre-1.0 GUI experiment while preserving the Rust CLI and Ratatui TUI as the canonical interfaces.

## [1.4.3] - 2026-05-11

### Changed
- Changed the crates.io package name to `tr300-tui` while keeping the product name SD-300 and the installed command `sd300`.
- Updated `sd300 update` so its Cargo strategy runs `cargo install tr300-tui --force`; installer fallback URLs now use the package-derived `tr300-tui-installer.*` cargo-dist assets while still installing the `sd300` binary.
- Clarified that `sd300`, `sd300 --user`, `sd300 --tech`, `sd300 update`, and legacy `sd300 --update` remain the standard user commands after installing from `tr300-tui`.
- Updated README, project context, project plan, local agent docs, and global Codex agent guidance to document `cargo install tr300-tui` as the supported Cargo install path.
- Kept the hand-edited WiX/MSI product name as `sd300` and allowed the MSI customization in cargo-dist config so the package rename does not rebrand the app installer.
- Kept only non-conflicting legacy `SD300-installer.sh` and `SD300-installer.ps1` aliases for older 1.4.0/1.4.1 installer fallback compatibility; lowercase `sd300-*` release-asset aliases were removed because GitHub release assets are case-sensitive in display but can conflict during upload.
- Bumped the release version to `1.4.3` for a clean crates.io and GitHub Release publish under the new package name.

## [1.4.2] - 2026-05-11

### Changed
- Switched the canonical crates.io package metadata to lowercase `sd300` so the supported Cargo install path is `cargo install sd300` while the installed command remains `sd300`.
- Removed the accidental uppercase `SD300` package from the release path and prepared the package for lowercase `sd300` publication.
- Updated `sd300 update` to use `cargo install sd300 --force` and lowercase cargo-dist installer asset URLs.
- Updated WiX/MSI product naming to lowercase `sd300`.
- Updated README, changelog, project context, project plan, local agent docs, and global Codex agent guidance for lowercase package, install, update, and release behavior.
- Release automation now publishes the crate after all cargo-dist artifacts build but before hosting the GitHub Release, reducing partial-release risk if crates.io rejects a publish.
- Release source-check can now repair a crates.io-published/GitHub-release-missing partial state by rebuilding artifacts and finishing release hosting.

### Added
- GitHub release uploads legacy `SD300-installer.sh` and `SD300-installer.ps1` aliases alongside lowercase installer assets so already-installed `1.4.0`/`1.4.1` updaters can still fall back to the installer path.

The `1.4.0` and `1.4.1` entries below are retained as historical notes for the
short-lived uppercase crates.io package path. `1.4.2` supersedes that path with
the lowercase `sd300` package metadata, and `1.4.3` supersedes it with the
publishable `tr300-tui` crates.io package while preserving the `sd300` command.

## [1.4.1] - 2026-05-11

### Changed
- Corrected Cargo installation documentation after release verification: the originally published crates.io package was `SD300`, `cargo install` required that package casing, and the installed command remained lowercase `sd300`.

## [1.4.0] - 2026-05-11

### Added
- `sd300 update` command form while preserving the legacy `sd300 --update` flag.
- Updater dispatch before Ratatui terminal initialization so update failures cannot leave the terminal in an altered TUI state.
- GitHub release updater that checks `QubeTX/qube-system-diagnostics` latest-release JSON, compares semantic version segments, exits `0` when current/successful, and exits `2` on update-check or update-attempt failure.
- Ordered updater strategies with per-attempt diagnostics:
  - Cargo first only when `cargo --version` succeeds, using `cargo install SD300 --force`.
  - macOS/Linux fallback through the cargo-dist shell installer with hardened `curl`, then `wget`.
  - Windows fallback through the cargo-dist PowerShell installer with `powershell.exe`, then `pwsh.exe`.
- New install and update documentation covering all supported installation paths: macOS/Linux shell installer (`SD300-installer.sh`), Windows PowerShell installer (`SD300-installer.ps1`), Windows MSI (`SD300-x86_64-pc-windows-msvc.msi`), Cargo (`cargo install SD300`), and source builds.
- Shared bounded command runner for collector subprocesses with timeout and kill behavior.
- Background startup and refresh jobs for connectivity, disk health, and driver scans so the TUI can render while slower probes run.
- CI workflow covering Ubuntu, macOS, and Windows with format checks, Clippy, tests, release build, target checks, audit, and `cargo-dist` plan.
- ND-300-style release workflow that can deploy from `main` when the current version is unreleased, while preserving explicit `v*.*.*` tag releases.
- Release source-check job that reads package metadata from `Cargo.toml`, checks crates.io version state, GitHub Release state, and remote tag state, skips fully published versions, and fails partial-release states before artifacts or crates are published.
- Initial crates.io publish job for the short-lived uppercase `SD300` release path; this was later superseded in `1.4.2` by the lowercase `sd300` package and publish-before-hosting release order.
- Tests for CLI update parsing/help/conflicts, updater strategy ordering and version comparison, bounded command timeout behavior, gateway/socket parser fixtures, and macOS disk/system-profiler parsers.
- `CODEX_PROJECT.md` project context file with current status and file tree.
- Local `AGENTS.md`, `CLAUDE.md`, and global Codex agent guidance documenting the SD300 release, publish, installation, and update workflows.

### Changed
- Bumped package version to `1.4.0` and crate package name to `SD300`; the installed binary remains `sd300`, and the Rust library target is `sd_300`.
- Added a crates.io package include list so published packages contain source, WiX manifest, Cargo metadata, changelog, license, README, and toolchain files without unrelated workspace files.
- Set Rust `1.95` as the explicit MSRV via `rust-version` and `rust-toolchain.toml`.
- Updated `sysinfo` to `0.39.x` and migrated to persistent `Networks`, `Disks`, and `Components` refresh handles.
- Updated direct `crossterm` to `0.29` to align with the Ratatui dependency tree.
- Removed the Rust HTTP client dependency from updater code; release metadata is fetched through bounded platform-native command helpers to avoid extra TLS/native build surface.
- Updated cargo-dist metadata for `SD300-*` release artifacts, shell/PowerShell/MSI installers, `CARGO_HOME` install path, and `allow-dirty = ["ci"]` because `.github/workflows/release.yml` is intentionally customized.
- Updated WiX packaging names, prompts, and install folder names from the old `sd-300` package identity to `SD300`.
- Network throughput now uses persistent refresh deltas instead of reconstructing network state each tick.
- Network interface display now includes operational state and uses Ratatui `Table`; bottom navigation now uses Ratatui `Tabs`.
- TUI sections now use more Ratatui-native composition and shared responsive helpers for bordered panels, gauges, tables, scroll indicators, and compact status rows.
- macOS audio driver collection now parses `system_profiler -json` instead of scanning JSON as plain text.
- macOS disk health parsing now has a dedicated `diskutil info` parser for model/media type detection.

### Fixed
- Update handling now happens before terminal initialization, avoiding dirty terminal state after update failures.
- Linux network operational states are normalized distinctly instead of treating every non-empty state as equivalent.
- External collector commands are bounded and degrade to unavailable/unknown data instead of freezing the app.
- Connectivity and disk-health refreshes no longer block the draw loop; they run as bounded background jobs and update warnings when complete.
- Windows Setup API driver scanning releases device-info handles through RAII cleanup even on early returns.
- macOS and Linux driver, disk, GPU, thermal, route, DNS, and socket probes use read-only commands with timeouts instead of unbounded `Command::output()` calls.
- Release metadata parsing now works on Ubuntu 22.04 runners without requiring Python `tomllib`.
- Clippy warnings blocking `-D warnings` were resolved across sorting, clamps, and key-event matching.

## [1.3.0] - 2026-03-12

### Added
- Man page generation via `clap_mangen` — `sd300.1` built automatically at compile time
- Enriched `--help` output with full keybindings table and all 9 diagnostic sections
- Scroll support for Drivers section (Tech Mode) — `j`/`k` keys with position indicator
- Scroll support for Disk section (Tech Mode) — `j`/`k` keys with position indicator
- `SPARK_SWAP` named color constant for swap sparkline consistency
- `Shift+M` keybinding for sorting processes by memory usage

### Changed
- Disk Tech Mode now uses bordered `sub_block()` panels for Partitions and Physical Drives (matches CPU/Memory layout)
- Network Tech Mode caps interface list at 8 entries with "+ N more" indicator (prevents layout overflow on Docker/WSL hosts)
- Label padding standardized to 18 chars in Memory section (was 20, now matches all other sections)
- Swap sparkline uses dedicated `SPARK_SWAP` constant instead of reusing `COLOR_WARN`
- Process sort header updated: `[m]emory` → `[M]emory` to reflect actual keybinding
- CPU per-core gauge width documented with clarifying comment (16-char fits 50% split at 80-col minimum)

### Fixed
- **Memory sort keybinding unreachable (BUG)**: `m` was bound to "return to mode selection" globally, making `ProcessSortKey::Memory` impossible to activate — now uses `Shift+M` which doesn't conflict

## [1.2.2] - 2026-03-12

### Changed
- Upgraded cargo-dist from v0.30.3 to v0.31.0 (includes bugfixes from v0.30.4 and installation robustness improvements)
- Updated GitHub Actions: checkout v4→v6, upload-artifact v4→v6, download-artifact v4→v7

## [1.2.1] - 2026-02-09

### Changed
- **Driver scanning replaced WMI with Windows Setup API**: Device enumeration now uses `SetupDi*` functions and Configuration Manager (`CM_Get_DevNode_Status`) instead of `Win32_PnPSignedDriver` WMI queries — immune to WMI repository corruption
- **Service status via Service Control Manager**: Replaced `Win32_Service` WMI query with direct SCM API (`OpenSCManager`/`QueryServiceStatus`) for the 13 monitored services
- Driver version and date now read directly from registry (`HKLM\SYSTEM\CurrentControlSet\Control\Class`) instead of WMI
- Renamed `DriverScanStatus::WmiUnavailable` to `ScanFailed` to reflect the API-agnostic implementation
- Updated error messages to remove WMI-specific language

### Added
- `windows` crate v0.62 dependency with Setup API, Registry, and Services features

## [1.2.0] - 2026-02-09

### Added
- Header bar with mode badge (User/Tech) and UTC clock across all screens
- `content_block()` and `sub_block()` helpers for consistent bordered panels (rounded borders)
- Temperature sparkline now visible in User Mode thermals (was Tech-only)
- Fan RPM display in User Mode thermals (was Tech-only)
- "Scanning..." animated state for driver tab during WMI scans
- Pipe separators (`|`) and right-aligned "? Help" hint in bottom navigation bar

### Changed
- **Complete UI overhaul**: Warm earth color palette replacing neon terminal colors
  - Sage green (good), warm amber (warnings), terracotta red (critical), warm gold (accent)
  - Slate blue (info), warm gray (dim/muted), warm white (text)
- All 9 tabs x 2 modes now use bordered content panels with rounded corners
- Gauge bars standardized to 20-character width across all sections, cleaner Unicode blocks
- Bottom bar: active tab uses warm gold on dark background, inactive tabs in muted gray
- Mode select screen: rounded borders, warm palette (sage for User, amber for Tech)
- Help overlay: warm palette with gold accent keys
- Overview User Mode: shows 5 top processes (was 3), wrapped in sub-panels
- Sparkline colors updated: warm gold (CPU), muted purple (memory), slate blue (network), sage (GPU), amber (temp)

### Fixed
- **Driver tab UI freeze (CRITICAL)**: WMI device scanning now runs asynchronously via `tokio::spawn_blocking` with `JoinHandle` polling, preventing 2-10s UI freezes
- Manual driver refresh ('r' key) no longer blocks the event loop
- WMI error messages now suggest running as Administrator
- Removed unused `Modifier` imports in cpu.rs and overview.rs

## [1.1.0] - 2026-02-08

### Added
- Ctrl+C to quit from any screen (OS-independent)
- Scroll indicators ("Showing X-Y of Z") on process table and network connections
- Network connections section documented in help overlay (j/k scroll hint)
- Temperature threshold constants (`TEMP_CPU_WARN/CRIT`, `TEMP_GPU_WARN/CRIT`) for consistent behavior

### Changed
- Extracted `truncate_str()` to shared `common.rs` (removed 8 duplicate copies)
- Consolidated `health_gauge_line()` and `health_gauge_line_simple()` into `common.rs`
- Replaced hardcoded refresh intervals with named constants (`REFRESH_FAST/SLOW/MEDIUM/DIAG/HEALTH`)
- Replaced hardcoded history buffer size with `HISTORY_SAMPLES` constant
- Fixed inconsistent temperature thresholds across overview, thermals, and CPU sections
- Tech mode sensor coloring now uses same thresholds as user mode (was 80/95, now 70/85)

### Fixed
- 13 Clippy warnings resolved (collapsible if, map_or to is_none_or, redundant to_string, useless format!, let_unit_value, manual range check)

## [1.0.0] - 2026-02-07

### Added
- Initial release with 9 diagnostic sections
- User Mode (plain language) and Technician Mode (raw data)
- Real-time monitoring: CPU, memory, disk, GPU, network, processes, thermals, drivers
- Cross-platform support: Windows, macOS, Linux (x86_64 + ARM)
- WMI-based driver scanning and SMART disk health (Windows)
- Network connectivity diagnostics (gateway, DNS, internet)
- Active connection monitoring with protocol/state/PID
- Temperature unit toggle (Celsius/Fahrenheit)
- cargo-dist release workflow with shell/powershell/MSI installers
