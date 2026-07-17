# SD-300 / SD300 Project Context

## TL;DR

SD-300 is a Rust/Ratatui cross-platform system diagnostics and monitoring TUI for Windows, macOS, and Linux. The binary is `sd300`; the crates.io package is `tr300-tui`, so users should install with `cargo install tr300-tui`.

Current state is version `1.4.3`, Rust `1.95`, `sysinfo` `0.39.x`, and `crossterm` `0.29`; it includes `sd300 update`, bounded external collector commands, background slow scans, CI, cargo-dist deployment, and crates.io publishing automation.

The 2026-07-17 M2 MacBook Pro research checkpoint documents the gap between the current macOS baseline and a comprehensive hardware monitor. The implementation-ready report is at [`docs/research/2026-07-17-macos-hardware-monitor-capability-report.md`](docs/research/2026-07-17-macos-hardware-monitor-capability-report.md). No runtime redesign has been applied yet; most implementation is intentionally reserved for the Alienware/Windows phase, with later physical-Mac validation gates.

## Current Status

- Primary CLI paths:
  - `sd300` opens interactive mode selection.
  - `sd300 --user` opens User Mode.
  - `sd300 --tech` opens Technician Mode.
  - `sd300 update` and `sd300 --update` run updater logic before terminal initialization.
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
  - cargo-dist release workflow is intentionally customized like ND-300: `main` pushes verify release state, build artifacts, publish crates.io, then host the GitHub release and installer assets.
  - The workflow can finish hosting if crates.io already has the exact version but the GitHub release is missing.
  - Release assets use cargo-dist's package-derived `tr300-tui-*` names; the shell, PowerShell, and MSI installers still install the `sd300` command.
  - Legacy uppercase `SD300-installer.*` aliases are kept only for 1.4.0/1.4.1 updater fallback compatibility. Do not add lowercase `sd300-*` aliases next to those uppercase aliases.
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
