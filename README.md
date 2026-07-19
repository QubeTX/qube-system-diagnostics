# SD-300 System Diagnostic

Real-time interactive TUI for system diagnostics and monitoring. Part of the **QubeTX 300 Series** alongside [TR-300](https://github.com/QubeTX/qube-machine-report) (Machine Report) and ND-300 (Network Diagnostic).

## Install

The managed CLI channel is recommended on every platform. Installer filenames
and public commands are stable and always resolve the latest qualified release.

### Windows (recommended)

```powershell
irm https://github.com/QubeTX/qube-system-diagnostics/releases/latest/download/sd300-cli-installer.ps1 | iex
```

### macOS and Linux (recommended)

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/QubeTX/qube-system-diagnostics/releases/latest/download/sd300-cli-installer.sh | sh
```

### Native installer options

- Windows Global MSI: `sd300-windows-x64-global.msi`
- Windows Corporate MSI (per-user, no admin): `sd300-windows-x64-corporate.msi`
- Windows Global EXE: `sd300-windows-x64-global.exe`
- Windows Corporate EXE (per-user, no admin): `sd300-windows-x64-corporate.exe`
- macOS universal signed package: `sd300-macos-universal.pkg`

Download them from the [latest release](https://github.com/QubeTX/qube-system-diagnostics/releases/latest). Global Windows installers use `%ProgramFiles%`; Corporate installers use `%LocalAppData%\Programs` and do not require elevation.

### Cargo (advanced/unmanaged)

```sh
cargo install tr300-tui
```

Published releases are available as the `tr300-tui` crate and still install the
lowercase `sd300` command. Raw Cargo has no post-install ownership hook, so it is
the advanced unmanaged channel. Use the recommended managed wrapper when you
want verified receipts, cross-method takeover, and deterministic uninstall.

After installation, the command surface is always `sd300`:

```sh
sd300
sd300 --user
sd300 --tech
sd300 update
sd300 --update
sd300 install
sd300 uninstall
sd300 snapshot --json
sd300 capabilities --json
```

### From Source

```sh
git clone https://github.com/QubeTX/qube-system-diagnostics.git
cd qube-system-diagnostics
cargo build --release
```

The binary will be at `target/release/sd300` (or `sd300.exe` on Windows).

## Features

SD-300 provides two diagnostic modes designed for different audiences:

**User Mode** presents system health in plain language with color-coded status indicators. Unsupported or unavailable telemetry remains explicit instead of being converted into a positive health claim.

**Technician Mode** exposes raw data: per-core CPU utilization, memory-module topology, driver versions and dates, network interface tables, and real-time sparkline graphs.

### Diagnostic Sections

Navigate between 9 sections using number keys:

| # | Section | User Mode | Technician Mode |
|---|---------|-----------|-----------------|
| 1 | **Overview** | System health dashboard | Identity, gauges, top processes |
| 2 | **CPU** | Load status, sparkline | Per-core bars, frequency, process table |
| 3 | **Memory** | Usage summary, top consumers | RAM/Swap sparklines, process table |
| 4 | **Disk** | Drive health, space usage | Mount table, filesystem details |
| 5 | **GPU** | Card status, utilization | VRAM, driver, utilization sparkline |
| 6 | **Network** | Connection status, speed | Interface table, throughput sparklines |
| 7 | **Processes** | Running apps in plain language | Sortable process table with scroll |
| 8 | **Thermals** | Temperature, fans, battery | Sensor table, fan RPM, battery details |
| 9 | **Drivers** | Device health overview | Driver versions, dates, service status |

### Live Updates

- **Fast metrics** (CPU, memory, network, processes): every 1 second
- **Slow metrics** (disk, GPU, thermals): every 5 seconds
- **Connections**: every 3 seconds
- **Connectivity**: background refresh every 15 seconds
- **Disk health**: background refresh every 60 seconds
- **Drivers**: background startup scan and on demand (press `r`)

## Usage

```
sd300              # Interactive mode selection
sd300 --user       # Launch directly into User Mode
sd300 --tech       # Launch directly into Technician Mode
sd300 update       # Check for and install the latest release
sd300 --update     # Legacy update flag
sd300 --help       # Show help
sd300 --version    # Show version
```

## Install, Update, and Uninstall Semantics

`sd300 update` proves the running binary's owner before changing anything, then
installs the latest release through that same channel: managed PowerShell,
managed shell, Cargo, Global/Corporate MSI, Global/Corporate EXE, or macOS PKG.
Ambiguous ownership fails before mutation. Native downloads and wrappers are
staged privately and verified against SHA-256 sidecars, including the exact-tag
cargo-dist payload used inside each managed wrapper. On Windows, updates rename
the running image to one tightly bounded rollback sibling before replacement;
Global MSI/EXE channels use one elevated same-channel worker so Restart Manager
cannot terminate the reporting parent or strand an unverified update.

If both Cargo and a managed receipt claim the same binary, the newer structured
ownership record wins. Equal timestamps or contradictory evidence fail closed
and direct the user to run a fresh official installer. JSON lifecycle responses
always include `recovery_url` and `requires_user_action` so automation can
distinguish a completed transaction from a safe handoff to the user.

`sd300 install` deliberately runs the preferred managed CLI installer. A fresh
official install is authoritative even when it is the same or an older version:
it removes only recognized prior SD-300 ownership after the replacement is
verified, and rolls back on failure. Direct native installers apply the same
policy within their scope and stop before mutation if an opposite Windows scope
is registered. `sd300 uninstall` delegates to the proven owner, removes its
binary, receipt or native registration, installer marker, and SD-300-only PATH
entry, and preserves unrelated Cargo/Rust tooling and shared PATH entries.
Windows native uninstall first retires the running image so MSI/EXE cleanup
cannot terminate the command before it reports the final result.

The legacy `sd300 --update` flag remains supported. Immutable 1.4.x fallback
filenames remain as compatibility routers so existing clients can cross the v2
cutover while preserving exact MSI/EXE/PKG ownership when it can be proven.

## Keybindings

| Key | Action |
|-----|--------|
| `1`-`9` | Switch to section |
| `q` / `Esc` | Quit |
| `Ctrl+C` | Quit to shell |
| `m` | Return to mode selection |
| `?` | Help overlay |
| `f` | Toggle temperature unit (C/F) |
| `j` / `k` | Scroll (processes, connections, drivers, disk in Tech Mode) |
| `c` / `M` / `n` / `p` | Sort by CPU / Memory / Name / PID (Section 7) |
| `r` | Manual refresh (Section 9 - Drivers) |

## Platform Support

| Platform | Target | Status |
|----------|--------|--------|
| Windows x86_64 | `x86_64-pc-windows-msvc` | Native install and diagnostics release gate |
| macOS x86_64 | `x86_64-apple-darwin` | Native Intel PKG release gate |
| macOS ARM | `aarch64-apple-darwin` | Native Apple Silicon PKG release gate |
| Linux x86_64 | `x86_64-unknown-linux-gnu` | Managed lifecycle release gate |
| Linux x86_64 (musl) | `x86_64-unknown-linux-musl` | Built and checksummed |
| Linux ARM | `aarch64-unknown-linux-gnu` | Built and checksummed |

### Platform-Specific Features

- **Windows**: Setup API plus WMI driver reconciliation, memory-module inventory, multi-GPU inventory, display topology/brightness, physical-disk health and explicit reliability availability, battery/power state, hardware identity, and native network link state/speed
- **Linux**: sysfs-based driver scanning, PCI device enumeration, ALSA audio detection
- **macOS**: bounded `system_profiler`/`diskutil`/network fallbacks plus `sysinfo`; the current implementation does not yet expose the full native hardware capability discovered on real M2 hardware

`sd300 snapshot --json` provides a noninteractive, privacy-redacted diagnostic
record; `sd300 capabilities --json` distinguishes available, unavailable,
unsupported, permission-denied, contradictory, and error states instead of
inventing zero-valued telemetry. Add `--include-sensitive` only when explicitly
needed for a local JSON snapshot.

## Screenshots

*Coming soon*

## License

PolyForm Noncommercial 1.0.0 - see [LICENSE.md](LICENSE.md).

Built by [QubeTX](https://github.com/QubeTX).
