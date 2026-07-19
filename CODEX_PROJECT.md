# SD-300 / SD300 Project Context

## TL;DR

SD-300 is a Rust/Ratatui cross-platform system diagnostics and monitoring TUI for Windows, macOS, and Linux. The binary is `sd300`; the crates.io package is `tr300-tui`. The recommended install is the stable managed wrapper (`irm | iex` on Windows, `curl | sh` on macOS/Linux); raw Cargo is an advanced unmanaged option.

Current state is version `2.0.3`, Rust `1.95`, `sysinfo` `0.39.x`, and `crossterm` `0.29`. It includes provenance-aware snapshots/capabilities, channel-preserving install/update/uninstall, expanded Windows diagnostics, stable managed/native artifacts, and draft-gated release qualification.

The 2026-07-17 M2 MacBook Pro research checkpoint remains the implementation boundary for deeper private/model-specific macOS telemetry. The v2 release adds explicit observation states and native Intel/Apple Silicon PKG gates without claiming that every research-only private signal is implemented.

## Current Status

- Primary CLI paths:
  - `sd300` opens interactive mode selection.
  - `sd300 --user` opens User Mode.
  - `sd300 --tech` opens Technician Mode.
  - `sd300 update` and `sd300 --update` run updater logic before terminal initialization.
  - `sd300 install` makes the preferred managed channel authoritative.
  - `sd300 uninstall` delegates to the proven owner.
  - `sd300 snapshot --json` and `sd300 capabilities --json` provide redacted automation surfaces.
- Core collector model:
  - Fast refresh: CPU, memory, network, processes.
  - Slow refresh: disk, GPU, thermals.
  - Medium refresh: active network connections.
  - Background jobs: connectivity, disk health, drivers.
- macOS research status:
  - Live-tested on a native arm64 `Mac14,7` M2 MacBook Pro running macOS 26.3.1.
  - Current collectors expose only a baseline and misclassify or omit multiple locally available signals.
  - The research report specifies public and private access tiers, exact Rust/FFI integration guidance, safe cadence/redaction, sanitized real payload examples, the macOS 26 non-guaranteed NVMe SMART-detail route, different-Mac qualification, and optional Vercel Labs Native GUI boundaries.
  - TUI and CLI remain the canonical product surfaces; the GUI is only a decoupled experiment.
- Packaging:
  - `release.yml` builds cargo-dist artifacts and creates an unpublished draft with stable managed wrappers and legacy compatibility routers.
  - Windows qualification builds/tests Global/Corporate MSI and EXE assets; macOS qualification signs, notarizes, and tests the universal PKG on native Intel and Apple Silicon runners.
  - Windows and macOS qualification compile a synthetic prior-version binary and prove real same-channel replacement, rollback cleanup, complete proven-owner CLI uninstall, and final version verification rather than only checking an already-current install.
  - `release-qualify.yml` verifies every updater-facing checksum, publishes the crate only after the native test matrix passes, publishes the draft as `latest`, and proves public Linux managed-shell and Cargo version transitions plus fresh install/uninstall.
  - Public native and wrapper filenames are stable and versionless. Internal exact-tag downloads remain immutable.
  - `sd300-cli-installer.*` is the advertised managed wrapper. `tr300-tui-installer.*` and `SD300-installer.*` are compatibility routers for immutable 1.4.x clients.
  - `allow-dirty = ["ci", "msi"]` is set in cargo-dist metadata because `.github/workflows/release.yml` and MSI product naming have deliberate deployment customizations.

## Goals

- Keep the monitor stable on all supported OSs and terminal environments.
- Prefer read-only, bounded platform probes that degrade to unknown/unavailable over blocking or crashing.
- Keep User Mode clear and nontechnical while preserving dense Technician Mode views.
- Preserve the `sd300` binary name even if package naming changes.
- Keep GitHub Actions artifact builds as the release gate before any crates.io publish.
- Build a provenance-first, capability-detected monitor where unsupported, absent, denied, stale, and failed readings are never converted into positive health claims.
- Preserve platform collectors behind a reusable Rust core so the same observations can serve CLI, TUI, safe exports, and an optional GUI.

## File Tree

```text
.
├── .github
│   └── workflows
│       ├── ci.yml
│       ├── claude-code-review.yml
│       ├── claude.yml
│       └── release.yml
├── .gitignore
├── AGENTS.md
├── CHANGELOG.md
├── CLAUDE.md
├── CODEX_PROJECT.md
├── Cargo.lock
├── Cargo.toml
├── LICENSE.md
├── README.md
├── SD300-Project-Plan.md
├── build.rs
├── docs
│   ├── research
│   │   └── 2026-07-17-macos-hardware-monitor-capability-report.md
│   └── thinking
│       └── 2026-07-17-macos-hardware-monitor-inquiry.md
├── rust-toolchain.toml
├── src
│   ├── app.rs
│   ├── cli.rs
│   ├── collectors
│   │   ├── command.rs
│   │   ├── cpu.rs
│   │   ├── disk.rs
│   │   ├── disk_health.rs
│   │   ├── drivers
│   │   │   └── platform
│   │   │       ├── linux.rs
│   │   │       ├── macos.rs
│   │   │       ├── mod.rs
│   │   │       └── windows.rs
│   │   ├── drivers.rs
│   │   ├── gpu.rs
│   │   ├── memory.rs
│   │   ├── mod.rs
│   │   ├── network.rs
│   │   ├── network_diag.rs
│   │   ├── platform
│   │   │   └── mod.rs
│   │   ├── processes.rs
│   │   ├── system_info.rs
│   │   └── thermals.rs
│   ├── error.rs
│   ├── history.rs
│   ├── lib.rs
│   ├── main.rs
│   ├── types.rs
│   ├── ui
│   │   ├── bottom_bar.rs
│   │   ├── common.rs
│   │   ├── header_bar.rs
│   │   ├── help_overlay.rs
│   │   ├── mod.rs
│   │   ├── mode_select.rs
│   │   └── sections
│   │       ├── cpu.rs
│   │       ├── disk.rs
│   │       ├── drivers.rs
│   │       ├── gpu.rs
│   │       ├── memory.rs
│   │       ├── mod.rs
│   │       ├── network.rs
│   │       ├── overview.rs
│   │       ├── processes.rs
│   │       └── thermals.rs
│   └── update.rs
└── wix
    └── main.wxs
```
