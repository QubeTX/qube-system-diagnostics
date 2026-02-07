# SD-300 System Diagnostic

Real-time interactive TUI for system diagnostics and monitoring. Part of the **QubeTX 300 Series** alongside [TR-300](https://github.com/QubeTX/qube-machine-report) (Machine Report) and ND-300 (Network Diagnostic).

## Installation

### Shell (macOS/Linux)

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/QubeTX/qube-system-diagnostics/releases/latest/download/sd-300-installer.sh | sh
```

### PowerShell (Windows)

```powershell
powershell -ExecutionPolicy ByPass -c "irm https://github.com/QubeTX/qube-system-diagnostics/releases/latest/download/sd-300-installer.ps1 | iex"
```

### Windows Installer (.msi)

Download `sd-300-x86_64-pc-windows-msvc.msi` from the [Releases](https://github.com/QubeTX/qube-system-diagnostics/releases) page.

### Cargo

```sh
cargo install sd-300
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

**User Mode** presents system health in plain language with color-coded status indicators. No technical knowledge required — statuses like "Running quietly", "Memory is getting full", and "Warm" replace raw numbers.

**Technician Mode** exposes raw data: per-core CPU utilization, exact memory addresses, driver versions and dates, network interface tables, and real-time sparkline graphs.

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
- **Drivers**: on demand (press `r`) or every 30 seconds

## Usage

```
sd300              # Interactive mode selection
sd300 --user       # Launch directly into User Mode
sd300 --tech       # Launch directly into Technician Mode
sd300 --help       # Show help
sd300 --version    # Show version
```

## Keybindings

| Key | Action |
|-----|--------|
| `1`-`9` | Switch to section |
| `q` / `Esc` | Quit |
| `m` | Return to mode selection |
| `?` | Help overlay |
| `f` | Toggle temperature unit (C/F) |
| `j` / `k` | Scroll (process list, Technician Mode) |
| `c` / `n` / `p` | Sort by CPU / Name / PID (Section 7) |
| `r` | Manual refresh (Section 9 - Drivers) |

## Platform Support

| Platform | Target | Status |
|----------|--------|--------|
| Windows x86_64 | `x86_64-pc-windows-msvc` | Full support |
| macOS x86_64 | `x86_64-apple-darwin` | Full support |
| macOS ARM | `aarch64-apple-darwin` | Full support |
| Linux x86_64 | `x86_64-unknown-linux-gnu` | Full support |
| Linux x86_64 (musl) | `x86_64-unknown-linux-musl` | Full support |
| Linux ARM | `aarch64-unknown-linux-gnu` | Full support |

### Platform-Specific Features

- **Windows**: WMI-based driver scanning, battery info via PowerShell, GPU via nvidia-smi
- **Linux**: sysfs-based driver scanning, PCI device enumeration, ALSA audio detection
- **macOS**: IOKit-based driver scanning, system_profiler integration

## Screenshots

*Coming soon*

## License

PolyForm Noncommercial 1.0.0 - see [LICENSE.md](LICENSE.md).

Built by [QubeTX](https://github.com/QubeTX).
