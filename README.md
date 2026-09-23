# SD-300 System Diagnostic

Cross-platform system diagnostics with an established Rust/Ratatui CLI/TUI and
an additive native desktop monitor. Part of the **QubeTX 300 Series** alongside
[TR-300](https://github.com/QubeTX/qube-machine-report) (Machine Report) and
ND-300 (Network Diagnostic).

SD-300 4.0.0 adds timestamped measurements, bounded background collection,
an adaptive terminal dashboard, a refined native GUI with Makira and Gail Rock,
and optional ND-300 diagnostics. Windows, macOS and Linux share the same Rust
collector core. Managed installers include the terminal tool and a clickable
app with the SD-300 icon. See the [qualification record](docs/qualification/v4/README.md)
for measured results, platform limitations and release-specific decisions.

## Monitoring and inspection

The monitoring dashboard adapts to an 80×24 terminal and expands into aligned charts,
tables and an inspector on wider terminals. All nine sections work in User and Technician
modes. `/` filters the full inventory, arrows or j/k select rows, PgUp/PgDn move a page,
and Enter opens details. Space freezes the view while monitoring continues; press it again
to show the newest capture. F explains findings and their evidence. Process sorts remain
c (CPU), M (memory), p (PID), n (name), with s reversing direction. Esc closes transient
panels or clears a filter before quitting. Charts preserve unobserved intervals as gaps.

Optional terminal preferences live in the settings document's independent `tui` object:
`mouse_enabled` (false), `reduced_motion` (true), `ascii` (false), `no_color` (false).
`NO_COLOR`, `SD300_ASCII`, and `TERM=dumb` are also honored. GUI preferences do not change
these defaults. Mouse wheel and row/tab clicks supplement complete keyboard navigation.
Technician inspection exposes units, availability, source, identity and capture age.

JSON exports retain schema 1 by default. Request `snapshot --json --schema-version 2`
or `capabilities --json --schema-version 2` for nullable readings, sample metadata and
shared findings. In a live TUI session, E opens the export panel; E there saves the
redacted snapshot and C saves capabilities. Reports include completed companion and
privileged-read results, preserve capture times, and never replace existing files.
A paused view exports its frozen samples. The GUI uses the same private report writer.
Unavailable or stale measurements remain explicit in both frontends.

## Install

The managed channel is recommended on every platform. Installer filenames
and public commands are stable and always resolve the latest qualified release.
Since v3.0.0, managed wrappers and native installers install the CLI/TUI and
desktop app as one product. Ordinary installation and update never launch the
GUI automatically; the one exception is the app's own "Update now" action,
which reopens the monitor only after a successful update (see
[Install, Update, and Uninstall Semantics](#install-update-and-uninstall-semantics)).

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
the advanced unmanaged CLI-only channel. Use the recommended managed wrapper
when you want the complete CLI+GUI product, verified receipts, cross-method
takeover, and deterministic uninstall.

Existing Cargo-owned v2 installations have an intentional two-step transition:

1. The first `sd300 update` uses the existing Cargo route to install the v3 CLI.
2. Run `sd300 update` again at the same version. The v3 CLI installs the complete
   managed CLI+GUI product and transactionally transfers ownership from Cargo.

After that takeover, future updates and uninstall use the managed channel. An
already complete managed installation remains a normal same-version no-op.

After installation, the command surface is always `sd300`:

```sh
sd300
sd300 --user
sd300 --tech
sd300 gui
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
Source builds produce the CLI/TUI only unless the separately pinned GUI build
and packaging pipeline is also run.

## Product Surfaces

- Bare `sd300` still opens the existing User/Technician chooser. The v3 GUI does
  not replace, wrap, or auto-launch from the terminal experience.
- `sd300 gui` launches or focuses the installed desktop app. If its companion is
  missing or corrupt, the command reports how to repair it with `sd300 install`
  or `sd300 update`; it does not fall back to a different UI silently.
- The GUI and TUI use the same Rust collectors, typed observations,
  capabilities, warning rules, provenance, and redaction. Frontend-specific
  rendering and runtime state remain isolated, and feature parity is a release
  invariant.
- The GUI exposes richer visual history, tables, details, and GUI-only
  preferences without changing any TUI startup choice, default, keybinding, or
  refresh cadence.
- Its Warm Carbon visual system uses black/charcoal depth, restrained orange
  energy, a subtle fading grid, Makira for headings and major numerals,
  Gail Rock for body copy and controls, and IBM Plex Mono for compact technical text.

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
sd300                    # Interactive mode selection
sd300 --user             # Launch directly into User Mode
sd300 --tech             # Launch directly into Technician Mode
sd300 gui                # Launch or focus the installed desktop monitor
sd300 update             # Check for and install the latest release
sd300 --update           # Legacy update flag
sd300 install            # Deliberate preferred managed install
sd300 uninstall          # Remove the product through its proven owner
sd300 snapshot --json    # Redacted noninteractive diagnostic snapshot
sd300 capabilities --json # Capability/provenance matrix
sd300 --help             # Show help
sd300 --version          # Show version
```

Updates can also be started from the desktop app itself: the Settings page's
"Update now" button and the tray's "Update SD-300" item run this same `sd300
update` transaction through a detached coordinator and reopen the app when it
succeeds.

## Install, Update, and Uninstall Semantics

`sd300 update` proves the running binary's owner before changing anything, then
installs the latest release through that same channel: managed PowerShell,
managed shell, Cargo, Global/Corporate MSI, Global/Corporate EXE, or macOS PKG.
Ambiguous ownership fails before mutation. Native downloads and wrappers are
staged privately and verified against SHA-256 sidecars, including the exact-tag
cargo-dist payload used inside each managed wrapper. On Windows, updates rename
the running image to one tightly bounded rollback sibling before replacement;
Global MSI/EXE channels use one elevated same-channel worker so Restart Manager
cannot terminate the reporting parent or strand an unverified update. In v3,
the selected artifact is composite: CLI and GUI payloads are verified before
mutation, the running GUI is asked to quit, both components are verified after
installation, and failure restores the prior CLI+GUI state together. A missing
GUI at the current version is a repair; a complete installation is still a
no-op. The GUI never opens as a side effect of install or update, with one
deliberate exception: an update started from the app's own Settings page or
tray menu runs this same CLI transaction through a detached coordinator and
reopens the monitor only after the transaction succeeds. An already-current
product keeps the running app open, and a failed update never launches it.

If both Cargo and a managed receipt claim the same binary, the newer structured
ownership record wins. Equal timestamps or contradictory evidence fail closed
and direct the user to run a fresh official installer. JSON lifecycle responses
always include `recovery_url` and `requires_user_action` so automation can
distinguish a completed transaction from a safe handoff to the user.

`sd300 install` deliberately runs the preferred managed installer. A fresh
official install is authoritative even when it is the same or an older version:
it removes only recognized prior SD-300 ownership after the replacement is
verified, and rolls back on failure. Direct native installers apply the same
policy within their scope and stop before mutation if an opposite Windows scope
is registered. `sd300 uninstall` delegates to the proven owner and removes the
owned CLI, GUI, engine, integration, launch-at-login entry, receipt or native
registration, installer marker, SD-300-only PATH entry, and private application
data. It preserves ambiguous paths, unrelated Cargo/Rust tooling, shared PATH
entries, and user-exported reports.
Windows native uninstall first retires the running image so MSI/EXE cleanup
cannot terminate the command before it reports the final result.

The legacy `sd300 --update` flag remains supported. Immutable 1.4.x fallback
filenames remain as compatibility routers so existing clients can cross the v2
cutover while preserving exact MSI/EXE/PKG ownership when it can be proven.

## GUI Settings, Tray, and Startup

The versioned settings document separates `shared`, `tui` and `gui` namespaces.
Verified optional-helper paths are deliberately shared; terminal presentation
preferences belong under `tui`. GUI mode, temperature unit, window geometry,
chart density, navigation, tray, close behavior, launch-at-login, and reduced motion remain in
`gui`. In particular, GUI choices never change the TUI chooser, sort defaults,
temperature default, or session behavior.

Settings live at `%APPDATA%\SD-300\settings.json` on Windows,
`~/Library/Application Support/SD-300/settings.json` on macOS, and
`${XDG_CONFIG_HOME:-~/.config}/sd300/settings.json` on Linux.

Tray and launch-at-login are independent. The GUI tray defaults on; startup
defaults off. On Windows and macOS, closing the window keeps the tray and
monitoring engine alive by default, Open restores the singleton window, Quit
terminates the GUI, and the tray tooltip carries a bounded live CPU, memory,
GPU, storage, and disk-health summary. The GUI setting “Keep monitoring after
closing the window” can make X quit the GUI and tray instead. Turning the tray
off applies on the next GUI launch. Bare `sd300`, `--user`, and `--tech` remain
tray-free because the TUI never starts the GUI process. Native SDK 0.5.4 does
not provide the required Linux tray, so Linux closes normally and
launch-at-login must open a visible window rather than leave an undiscoverable
background process.

The GUI provides keyboard navigation, visible focus, text equivalents for
charts, reduced motion, and an internally audited semantic tree on every target.
With the pinned Native SDK 0.5.4, individual retained canvas controls reach the
operating-system accessibility tree on macOS only; Windows and Linux expose the
named application canvas but not its child controls. Use the unchanged terminal
UI when system screen-reader navigation is required on those platforms. This is
a documented SDK limitation, not a successful Windows/Linux screen-reader claim.

## Keybindings

| Key | Action |
|-----|--------|
| `1`-`9` | Switch to section |
| `q` / `Esc` | Quit |
| `Ctrl+C` | Quit to shell |
| `m` | Return to mode selection |
| `?` | Help overlay |
| `f` | Toggle temperature unit (C/F) |
| `j` / `k` | Scroll (processes, connections, drivers; disk in Tech Mode) |
| `c` / `M` / `n` / `p` | Sort by CPU / Memory / Name / PID (Section 7) |
| `r` | Manual refresh (Section 9 - Drivers) |

## Platform Support

| Platform | Target | Composite release requirement |
|----------|--------|------------------------|
| Windows x86_64 | `x86_64-pc-windows-msvc` | CLI/TUI + GUI + managed/native lifecycle |
| macOS x86_64 | `x86_64-apple-darwin` | CLI/TUI + GUI in universal PKG, native Intel qualification |
| macOS ARM64 | `aarch64-apple-darwin` | CLI/TUI + GUI in universal PKG, native Apple Silicon qualification |
| Linux GNU x86_64 | `x86_64-unknown-linux-gnu` | CLI/TUI + GUI + private runtime + managed lifecycle |
| Linux GNU ARM64 | `aarch64-unknown-linux-gnu` | CLI/TUI + GUI + private runtime + managed lifecycle |
| Linux musl x86_64 | `x86_64-unknown-linux-musl` | CLI/TUI + GUI + private runtime + managed lifecycle |

These are the six exact release targets. The x86_64 artifacts cover both Intel
and AMD processors. Windows ARM64 and Linux musl ARM64 are not implied by the
word “ARM” and are not part of this release matrix.

## Performance and Release Trust

The GUI keeps collector sampling live while publishing bounded, latest-only
projections so a slow renderer cannot create work backlog. In a visible window,
fast-topic samples must reach the GUI at least once per second after renderer
optimization; hidden/tray mode may coalesce work to the summary data it needs.
Release builds must pass Native SDK strict checks, lifecycle and compatibility
matrices, path-leak
scans, and foreground/hidden/soak performance tests. The v3 budgets are at most
2% of one logical core foreground, 1% hidden/tray, 150 MiB working set/RSS,
300 MiB private memory/commit, 16.7 ms frame-time p95, and 50 ms input-response
p95 outside explicit scans, with no unbounded history, event, log, or memory
growth. For 4.0.0 only, the operator approved 4% foreground CPU, 3% hidden CPU,
200 MiB RSS, and frame/input p95 up to 100 ms; private memory and the 100 ms
ordinary-refresh maximum are unchanged. The [original goals](docs/next-version-targets.md)
automatically return for the next version. Retained reports distinguish original
target verdicts from the approved release policy.

Qualified release assets include SHA-256 sidecars, an SPDX SBOM, and GitHub
artifact attestations. After a public release, verify a downloaded asset with:

```sh
gh attestation verify <asset> -R QubeTX/qube-system-diagnostics
```

Attestation proves the repository, workflow, commit, and artifact digest that
produced the bytes. It is not a commercial code-signing identity and does not
eliminate Windows SmartScreen warnings.

### Platform-Specific Features

- **Windows**: authoritative Setup API/Config Manager driver health, memory-module inventory, multi-GPU inventory and NVIDIA telemetry, display topology/brightness, physical-disk health and explicit reliability availability, battery/power state, hardware identity, native network link state/speed, hardware-monitor WMI bridges, and guarded read-only Dell AWCC thermal enumeration
- **Linux**: sysfs-based driver scanning, PCI device enumeration, ALSA audio detection
- **macOS**: bounded `system_profiler`/`diskutil`/network fallbacks plus `sysinfo`; the current implementation does not yet expose the full native hardware capability discovered on real M2 hardware

`sd300 snapshot --json` provides a noninteractive, privacy-redacted diagnostic
record; `sd300 capabilities --json` distinguishes available, unavailable,
unsupported, permission-denied, contradictory, and error states instead of
inventing zero-valued telemetry. Add `--include-sensitive` only when explicitly
needed for a local JSON snapshot.

Temperature capabilities are reported independently for CPU, GPU, aggregate
temperature, and fans. Windows first consumes native/component data and supported
Libre/Open Hardware Monitor WMI bridges, then uses read-only Dell AWCC firmware
operations when present, with ACPI as a fallback. Some low-level or vendor sensors
require an elevated provider; SD-300 reports that permission boundary and continues
showing any independently available GPU temperature instead of treating all thermal
telemetry as unsupported.

## Screenshots

*Coming soon*

## License

PolyForm Noncommercial 1.0.0 - see [LICENSE.md](LICENSE.md).

Built by [QubeTX](https://github.com/QubeTX).


### Optional network companion

The terminal's **N** panel and the app's **Network** section can run the installed
public ND-300 4.0.1 companion. Standard and deep scans explicitly skip speed
tests. SpeedQX is a separate bandwidth-consuming action with a displayed budget
and a fresh, optional M-Lab consent choice. Monitoring continues during scans;
cancellation never invokes network repairs. Unknown companion versions are
reported separately. Results stay in memory until exported, and redacted exports
omit imported free-form details.

Optional setup is a separate confirmation in both interfaces. The TUI uses **i**
inside the **N** panel; the GUI provides **Install optional ND-300**. The CLI
`sd300 tools nd300` describes the exact operation; `--install --accept` consents.
Setup uses the official release archive with a platform-specific pinned checksum,
keeps existing installations, and verifies both executables. The independent
per-user ND-300 directory survives SD-300 removal and carries its own standalone
receipt. No PATH changes, elevated install, diagnostic, repair or bandwidth test
is implied by setup. Optional-tool ownership is independent of SD-300 removal.


Optional SMART helper setup is available with **h** in the terminal's companion
panel, the app's **Install optional SMART helper** action, or
`sd300 tools smartctl --install --accept`. Inspect `sd300 tools smartctl` first
for the platform's exact operation. Windows uses a checksum-pinned, unelevated
component extraction; macOS uses existing Homebrew; Debian/Ubuntu and Alpine
extract authenticated packages without running their service scripts. Missing
runtime dependencies or unsupported package managers leave ordinary monitoring
available with a specific setup instruction. Setup does not authorize a device
self-test, repair, disk setting change, or privileged read.

A privileged SMART read is a separate operation. In the TUI, select a physical
drive under Storage and press **A**; in the GUI, use **Review read…** in Storage.
Review the device, helper path and checksum, then explicitly allow that read.
Only a short-lived worker requests OS authorization. Monitoring stays
unprivileged and responsive. The read has a twelve-second limit, authentication
expires after one minute, and cancellation preserves the previous result.
Linux needs PolicyKit and a registered graphical authentication agent; a
headless session retains ordinary unprivileged monitoring. No repair, self-test,
service or disk-setting change is performed. Captured results remain in memory
and appear in explicit schema-2 exports with identifier redaction.
