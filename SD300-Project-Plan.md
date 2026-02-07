# SD300 — System Diagnostic 300

## Project Plan & Developer Specification

**Product Line:** QubeTX Developer Tools (300 Series)
**Sibling Tools:** TR-300 (Machine Report), ND-300 (Network Diagnostic)
**Language:** Rust
**TUI Framework:** [Ratatui](https://github.com/ratatui/ratatui) (the actively maintained `tui-rs` successor)
**License:** PolyForm Noncommercial (consistent with TR-300)
**Target Platforms:** Windows, macOS, Linux — single static binary per platform, fully independent (zero runtime dependencies, no admin/root required for core diagnostics)

---

## 1. Overview

SD300 is a live, interactive terminal user interface (TUI) for real-time system diagnostics and monitoring. Unlike TR-300 (a one-shot snapshot) and ND-300 (a one-shot network diagnostic), SD300 is a persistent, interactive application that runs in the terminal and updates continuously.

SD300 must be fully cross-platform: a single codebase producing native binaries for Windows, macOS, and Linux. All core sections (CPU, Memory, Disk, GPU, Network, Processes, Thermals) must work on all three platforms, with graceful degradation where OS-level APIs differ. The **Drivers & Devices** section (Section 9) is the most platform-divergent — it surfaces full driver metadata on Windows, kernel module and service health on Linux, and device/framework status on macOS. Each platform gets the diagnostics that are relevant and meaningful to it, using native terminology. The tool detects the host OS at runtime and silently adapts — the user never needs to specify their platform.

The defining feature of SD300 is its **dual-mode system:**

- **User Mode** — Designed for everyday, non-technical users. Presents system health in plain language, simple layouts, and friendly descriptions. No raw numbers like clock speeds, voltages, or technical jargon. Still detailed and informative — but translated into language anyone can understand.
- **Technician Mode** — Designed for IT professionals and power users. Exposes raw data, advanced metrics, technical terminology, and dense dashboards. Everything a tech would want to see when diagnosing a machine.

The user selects their mode at launch. The mode determines the vocabulary, layout density, and level of detail displayed — but both modes have access to the same underlying diagnostic data.

---

## 2. Launch Experience — Mode Selection

When the user runs `sd300` with no arguments, the application opens with a **mode selection screen** before entering the main TUI.

### 2.1 Mode Selection Screen

This should be a simple, clean, full-screen prompt rendered in the TUI. Not a CLI flag — an interactive selection within the application itself.

**Conceptual layout:**

```
+===========================================================+
|                                                           |
|                SD-300 SYSTEM DIAGNOSTIC                   |
|                QubeTX Developer Tools                     |
|                                                           |
|  Select a diagnostic mode:                                |
|                                                           |
|    [1]  👤  User Mode                                     |
|         Plain language system health overview.             |
|         Designed for everyday users.                       |
|                                                           |
|    [2]  🔧  Technician Mode                               |
|         Advanced metrics and raw system data.              |
|         Designed for IT professionals.                     |
|                                                           |
|  Press 1 or 2 to continue.                                |
|                                                           |
+===========================================================+
```

- Pressing `1` enters User Mode.
- Pressing `2` enters Technician Mode.
- There should also be CLI flags to skip this screen: `sd300 --user` and `sd300 --tech` to launch directly into a mode.

The mode selection screen should be visually polished — it's the first thing anyone sees.

---

## 3. Navigation — Section System

Both modes use **numbered sections** that the user can switch between using the **number keys** on their keyboard. This is the primary navigation mechanism.

### 3.1 Section Map

The sections are the same in both modes (the content and presentation differ, but the categories are shared):

| Key | Section              | What It Covers                                                      |
|-----|----------------------|---------------------------------------------------------------------|
| `1` | **Overview**         | High-level system health dashboard. The "home" screen.              |
| `2` | **CPU**              | Processor diagnostics — load, temps, cores, frequency.              |
| `3` | **Memory**           | RAM usage, swap, per-process memory, pressure.                      |
| `4` | **Disk**             | Storage devices, usage, I/O rates, health (SMART if available).     |
| `5` | **GPU**              | Graphics card info, utilization, VRAM, temps (if detectable).       |
| `6` | **Network**          | Interface status, throughput, connections.                          |
| `7` | **Processes**        | Running processes, resource usage, top consumers.                   |
| `8` | **Thermals & Power** | Temperature sensors, fan speeds, battery (if laptop), power state.  |
| `9` | **Drivers & Devices**| Driver health for network, Bluetooth, audio, input devices. **(Windows primary; macOS/Linux show equivalent device/module info where applicable.)** |

### 3.2 Navigation UX

- A **persistent bottom bar** should always be visible, showing the available sections and which one is active. Something like:

  ```
  [1] Overview [2] CPU [3] Mem [4] Disk [5] GPU [6] Net [7] Procs [8] Thermals [9] Drivers
  ```

  The active section should be highlighted (bold, inverted, underlined, or color-highlighted). Section labels may be abbreviated in the bar to fit terminal width (as shown above).

- Pressing a number key instantly switches to that section. No menus, no enter key, no delays. Just press `3` and you're looking at Memory.
- Press `q` or `Esc` to quit the application.
- Press `m` to return to the mode selection screen (switch modes without restarting).
- Press `?` to show a help overlay with keybindings.

### 3.3 Live Updates

All sections update in real time. The TUI should refresh data at a sensible interval:

- **Fast metrics** (CPU load, network throughput, memory usage): every 1 second.
- **Slow metrics** (disk usage, SMART data, GPU info): every 5–10 seconds.
- **Static info** (hardware model, OS version, CPU model): fetched once at startup.

The refresh should be non-blocking. The UI must remain responsive while data is being collected.

---

## 4. User Mode — Detailed Design

User Mode is the heart of what makes SD300 different from `htop`, `btop`, or any other system monitor. It's specifically designed for people who are NOT technicians.

### 4.1 Philosophy & Language Guidelines

**Core principle:** Inform without intimidating. Every piece of information should be understandable by someone who knows how to use a computer but doesn't know what a "thread" or "clock speed" is.

**Vocabulary rules for User Mode:**

| Don't Say                            | Say Instead                                      |
|--------------------------------------|--------------------------------------------------|
| CPU at 4.5 GHz                       | Processor speed: Fast / Normal / Slowed down     |
| 87°C                                 | Processor temperature: Warm (working hard)        |
| 76% memory utilization               | Memory: Using most of what's available (76%)      |
| 12.4 GB / 16 GB RAM                  | 12 of 16 GB of memory in use                     |
| Swap usage: 2.1 GB                   | Your computer is using extra temporary storage     |
| Disk I/O: 142 MB/s read              | Storage: Currently reading data quickly            |
| PID 4821                             | (Just omit this entirely)                         |
| SMART status: PASSED                 | Drive health: Good                                |
| NVMe / SATA / HDD                    | Storage type: Fast solid-state / Mechanical drive  |
| eth0 / wlan0                         | Wired connection / Wi-Fi                           |
| 142.5 MB/s throughput                | Network speed: Fast                                |
| Packet loss: 0.2%                    | Connection quality: Stable                         |

**Percentages are fine.** Users understand "76%." What they don't understand is "76% of 16,384 MB across 2 DIMMs in dual-channel DDR5-5600 configuration."

**Context is key.** Don't just say "76%." Say "Using most of what's available (76%). This is normal when you have several apps open." Give the user a frame of reference.

**Color and visual cues matter.** Green = healthy/normal. Yellow = worth noting. Red = something needs attention. Use color bars, gauges, and indicators heavily in User Mode.

### 4.2 User Mode — Overview Screen (Section 1)

The Overview screen is the default landing screen after mode selection. It should give a complete health picture on a single screen.

**Conceptual layout:**

```
  SD-300 SYSTEM DIAGNOSTIC — User Mode
  ─────────────────────────────────────────────────

  SYSTEM HEALTH                        Right Now
  ─────────────────────────────────────────────────

  Processor     ✓ Running normally        [████████░░] 78%
  Memory        ✓ Plenty available        [██████░░░░] 58%
  Storage       ✓ Plenty of space         [████░░░░░░] 42%
  Graphics      ✓ Running normally        [██░░░░░░░░] 15%
  Network       ✓ Connected (Wi-Fi)       ↓ 12 Mbps  ↑ 3 Mbps
  Temperature   ✓ Normal                  Comfortable
  Drivers       ✓ All devices working

  ─────────────────────────────────────────────────
  WHAT'S USING THE MOST RESOURCES

  Google Chrome          Memory: ████░░  34%
  Spotify                Processor: ██░░░░  8%
  Windows Update         Storage: Writing

  ─────────────────────────────────────────────────
  YOUR COMPUTER

  Windows 11 Home  •  Dell Inspiron 15  •  Up 3 days
  Processor: AMD Ryzen 7  •  Memory: 16 GB  •  Storage: 512 GB SSD

  [1] Overview [2] CPU [3] Mem [4] Disk [5] GPU [6] Net [7] Procs [8] Thermals [9] Drivers
```

**Key design points for the overview:**
- Every line leads with a clear status icon (✓, ⚠, ✗).
- Short, natural language descriptions. "Plenty available." "Running normally." "Connected (Wi-Fi)."
- Visual bars give instant proportional understanding.
- Top resource consumers are shown by name (app name, not process name where possible — "Google Chrome" not "chrome.exe").
- Basic system identity at the bottom — enough for the user to tell a technician what they have.

### 4.3 User Mode — Section Screens (Sections 2–8)

Each section screen dives deeper into one category, still in plain language.

**Example: CPU section (key 2) in User Mode:**

```
  PROCESSOR
  ─────────────────────────────────────────────────

  Status         ✓ Running normally
  How busy       [████████░░] 78% — Fairly busy right now
  Temperature    Warm — This is expected when busy
  Speed          Running at full speed

  ─────────────────────────────────────────────────
  OVER TIME (last 60 seconds)

  100%|
   75%|    ▄▄██
   50%|  ▄█████▄▄     ▄▄
   25%| ██████████▄▄▄████▄▄
    0%|▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁
      60s ago                        now

  ─────────────────────────────────────────────────
  WHAT'S KEEPING THE PROCESSOR BUSY

  Google Chrome             ██████░░░░  28%
  Microsoft Teams           ████░░░░░░  14%
  Windows Defender Scan     ███░░░░░░░  12%
  Spotify                   █░░░░░░░░░   4%
  Everything else                        20%
```

**Same principles across all section screens in User Mode:**
- Lead with a plain-English status.
- Use visual charts (sparklines, bar charts, gauges) rendered in the TUI.
- Show a time-series graph where applicable (CPU load over time, memory over time, network throughput over time).
- Name applications by their user-visible name, not the process binary name.
- Use contextual descriptions: "Warm — This is expected when busy" rather than just "72°C."

---

## 5. Technician Mode — Detailed Design

Technician Mode is the "raw" experience. Dense, data-rich, and technically precise.

### 5.1 Philosophy & Language Guidelines

**Core principle:** Maximum information density. Show everything. Use correct technical terminology. Trust the technician to interpret the data.

Technician Mode should feel like a premium version of `btop` or `htop` with QubeTX's visual polish. Data tables, multi-panel layouts, raw numbers with units, and no hand-holding.

### 5.2 Technician Mode — Overview Screen (Section 1)

The overview packs as much data onto one screen as possible.

**Conceptual layout:**

```
  SD-300 SYSTEM DIAGNOSTIC — Technician Mode               14:32:07 CST
  ──────────────────────────────────────────────────────────────────────

  OS       Windows 11 Pro 24H2 (26200.5516)    Uptime   3d 14h 22m
  Host     Dell Inspiron 15 5530               Shell    PowerShell 7.5
  CPU      AMD Ryzen 7 7840HS (16) @ 5.1 GHz  Arch     x86_64
  GPU      NVIDIA RTX 4060 Mobile 8GB          Driver   566.14
  Memory   12.4 / 16.0 GiB (77.5%)            Swap     0.8 / 8.0 GiB
  ──────────────────────────────────────────────────────────────────────

  CPU [████████████████░░░░] 78.2%    MEM [████████████░░░░░░░░] 58.1%
  GPU [███░░░░░░░░░░░░░░░░░] 14.7%    SWP [██░░░░░░░░░░░░░░░░░░] 10.0%

  DSK C: [████████░░░░░░░░░░░░] 42%  847G/1.8T    R: 142 MB/s  W: 23 MB/s
  DSK D: [████████░░░░░░░░░░░░] 44%  3.2T/7.3T    R:   0 MB/s  W:  1 MB/s

  NET wlan0  ↓ 12.4 MB/s  ↑ 3.1 MB/s   192.168.1.42   SSID: HomeNetwork

  ──────────────────────────────────────────────────────────────────────
  TOP PROCESSES                    PID     CPU%    MEM%    MEM
  chrome.exe                      4821    28.1%   12.4%   1.99 GiB
  teams.exe                       3102    14.2%    5.1%   0.82 GiB
  MsMpEng.exe                     1847    12.0%    2.3%   0.37 GiB
  spotify.exe                     6234     3.9%    3.8%   0.61 GiB

  DRVRS   Net: OK  BT: OK  Audio: OK  Input: OK   (2 disabled adapters)
  TEMPS   CPU: 72°C (Tctl)   GPU: 58°C   SSD: 41°C   Fan: 3200 RPM

  [1] Overview [2] CPU [3] Mem [4] Disk [5] GPU [6] Net [7] Procs [8] Thermals [9] Drivers
```

### 5.3 Technician Mode — Section Screens (Sections 2–8)

Each section is a deep dive with full technical data.

**Example: CPU section (key 2) in Technician Mode:**

```
  CPU — AMD Ryzen 7 7840HS (16 threads / 8 cores) — x86_64
  ──────────────────────────────────────────────────────────────────────

  Total Load    [████████████████░░░░] 78.2%       Frequency   4.87 GHz
  Temperature   72°C (Tctl/Tdie)                   TDP         54W
  ──────────────────────────────────────────────────────────────────────

  PER-CORE UTILIZATION

  Core 0  [██████████████████░░] 89%  4.91 GHz  68°C
  Core 1  [████████████████░░░░] 81%  4.88 GHz  70°C
  Core 2  [██████████████░░░░░░] 72%  4.85 GHz  71°C
  Core 3  [████████████████████] 97%  5.10 GHz  74°C
  Core 4  [████████████░░░░░░░░] 62%  4.82 GHz  66°C
  Core 5  [██████████████████░░] 88%  4.90 GHz  72°C
  Core 6  [██████████░░░░░░░░░░] 51%  4.79 GHz  64°C
  Core 7  [████████████████░░░░] 78%  4.86 GHz  69°C

  ──────────────────────────────────────────────────────────────────────
  LOAD HISTORY (60s)

  100%|         ▄▄
   75%|    ▄▄████████▄
   50%|  ▄████████████████▄▄     ▄▄
   25%| ████████████████████████████████
    0%|▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁
      -60s                           now

  ──────────────────────────────────────────────────────────────────────
  TOP CPU CONSUMERS          PID     CPU%     THREADS   PRIORITY
  chrome.exe                 4821    28.1%    47        Normal
  MsMpEng.exe                1847    12.0%    12        Below Normal
  teams.exe                  3102    14.2%    31        Normal
  dwm.exe                    892      3.2%     8        High
```

**Technician Mode should include (across all sections):**
- Raw values with proper units (GHz, GiB, MB/s, °C, RPM, etc.).
- Per-component breakdowns (per-core CPU, per-DIMM memory if detectable, per-disk I/O).
- Process IDs, thread counts, priorities.
- Driver versions, firmware versions where available.
- SMART data for disks (if accessible).
- Full network interface details (MAC, MTU, IPv4/IPv6 addresses, packet counts, error counts).
- Historical sparkline/graph for any metric that changes over time.

---

## 6. Detailed Section Specifications

### 6.1 Section 2 — CPU

| Data Point           | User Mode                                      | Technician Mode                          |
|----------------------|------------------------------------------------|------------------------------------------|
| Model                | "AMD Ryzen 7" (simplified)                     | Full model string, stepping, arch        |
| Load                 | "Fairly busy (78%)" + bar                      | 78.2% + per-core breakdown              |
| Frequency            | "Running at full speed" / "Slowed down"        | Current GHz per core, base/boost clocks  |
| Temperature          | "Warm — expected when busy"                    | Exact °C per sensor, Tctl/Tdie           |
| Top consumers        | App names + bars                               | Process names, PIDs, CPU%, thread count  |
| History graph        | Simple 60s sparkline                           | Detailed 60s chart, optionally 5m/15m    |

### 6.2 Section 3 — Memory

| Data Point           | User Mode                                      | Technician Mode                          |
|----------------------|------------------------------------------------|------------------------------------------|
| Usage                | "Using 12 of 16 GB (76%)" + bar               | 12.41 / 16.00 GiB + breakdown           |
| Swap                 | "Using extra temp storage" (if active)         | Swap usage, swap I/O rate                |
| Status               | "Normal" / "Getting full" / "Almost out"       | Pressure metrics, page faults/sec        |
| Top consumers        | App names + "Using a lot" / "Normal"           | Process names, PIDs, RSS, VMS, Shared    |
| History graph        | Simple 60s usage trend                         | Detailed usage + swap trend              |

### 6.3 Section 4 — Disk / Storage

| Data Point           | User Mode                                      | Technician Mode                          |
|----------------------|------------------------------------------------|------------------------------------------|
| Capacity             | "Using 847 GB of 1.8 TB (42%)" + bar          | Per-partition, filesystem type, mount     |
| Type                 | "Fast solid-state drive"                       | NVMe / SATA SSD / HDD, model, firmware   |
| Health               | "Drive health: Good"                           | SMART attributes, error counts, hours     |
| I/O                  | "Reading/writing data now"                     | MB/s read/write, IOPS, queue depth       |
| History graph        | Simple I/O activity indicator                  | Read/write throughput over time           |

### 6.4 Section 5 — GPU

| Data Point           | User Mode                                      | Technician Mode                          |
|----------------------|------------------------------------------------|------------------------------------------|
| Model                | "NVIDIA graphics card"                         | Full model, VRAM, driver version, CUDA   |
| Utilization          | "Not very busy (15%)" + bar                   | GPU%, memory%, encoder/decoder%          |
| VRAM                 | "Graphics memory: Mostly free"                 | Used / Total VRAM in MiB                 |
| Temperature          | "Cool"                                         | Exact °C, fan speed                      |
| Top consumers        | App names using GPU                            | Process names, PIDs, VRAM per process    |

**Note on GPU data:** GPU telemetry is notoriously platform-specific. NVIDIA exposes data via NVML/`nvidia-smi`, AMD via ROCm-SMI or AMDGPU sysfs, Intel via `intel_gpu_top`. The developer should implement what's feasible and degrade gracefully (show "GPU data not available" if no supported GPU is detected). At minimum, target NVIDIA on all platforms and integrated GPUs where `sysinfo` exposes them.

### 6.5 Section 6 — Network

| Data Point           | User Mode                                      | Technician Mode                          |
|----------------------|------------------------------------------------|------------------------------------------|
| Connection           | "Connected via Wi-Fi" / "Wired connection"     | Interface name, type, MAC, MTU           |
| Adapter status       | "All network adapters working"                 | Per-adapter status (see Section 9 for full driver detail) |
| Speed                | "Downloading at 12 Mbps"                       | Bytes/sec in and out, packet counts      |
| Status               | "Connection stable" / "Some packet loss"       | Error counts, dropped packets, collisions|
| IP                   | "Connected to your home network"               | Local IP, subnet, gateway, DNS servers   |
| Wi-Fi                | "Signal: Strong (5 GHz)"                       | SSID, BSSID, signal dBm, channel, band  |
| History graph        | Simple throughput sparkline                    | Upload/download throughput over time     |

**Note:** The Network section shows live throughput and connection status. For deeper driver/adapter diagnostics (driver versions, service health, disabled adapters), the user should navigate to Section 9 — Drivers & Devices. The Network section may include a brief one-line cross-reference: "Press 9 for driver details" or similar.

### 6.6 Section 7 — Processes

| Data Point           | User Mode                                      | Technician Mode                          |
|----------------------|------------------------------------------------|------------------------------------------|
| List                 | Top 10 apps by name, sorted by resource use    | Full sortable process list               |
| Info per process     | App name + "Using a lot of memory" etc.        | PID, CPU%, MEM%, RSS, VMS, threads, user |
| Sorting              | Pre-sorted by "most demanding"                 | Sortable by any column (key shortcuts)   |
| Process count        | "142 things running"                           | "142 processes, 1847 threads"            |

**In Technician Mode, the process list should support:**
- Sorting by pressing `c` (CPU), `m` (memory), `p` (PID), `n` (name) — or similar keybindings.
- Scrolling through the full list with arrow keys or `j`/`k`.
- These sub-controls should be shown in the footer when this section is active.

### 6.7 Section 8 — Thermals & Power

| Data Point           | User Mode                                      | Technician Mode                          |
|----------------------|------------------------------------------------|------------------------------------------|
| CPU temp             | "Processor: Warm"                              | Exact °C per sensor                      |
| GPU temp             | "Graphics: Cool"                               | Exact °C                                 |
| Disk temp            | "Storage: Normal"                              | Exact °C per drive                       |
| Fan speed            | "Fans: Running" / "Fans: Off"                  | RPM per fan                              |
| Battery (if laptop)  | "Battery: 68%, about 3 hours left"             | Wh, charge rate, cycle count, health%    |
| Power state          | "Plugged in" / "On battery"                    | AC/DC state, wattage draw if available   |
| History graph        | Simple temperature trend                       | Multi-line temp chart for all sensors    |

### 6.8 Section 9 — Drivers & Devices

This section scans and displays the health status of hardware drivers and device subsystems. It covers four major device categories: **Network, Bluetooth, Audio, and Input (keyboard/mouse/trackpad)**. This is one of the most platform-divergent sections in SD300 — each OS exposes device and driver information very differently.

**Cross-platform design principle:** The section is always available on all platforms, but the depth of information and the terminology adjust per OS. On Windows, this is a full driver health dashboard. On macOS and Linux, it focuses on device presence, module/extension status, and service health — using the equivalent concepts for those platforms rather than forcing the Windows "driver" framing onto OSes where it doesn't apply.

| Data Point              | User Mode                                         | Technician Mode                                                     |
|-------------------------|----------------------------------------------------|----------------------------------------------------------------------|
| **Overall status**      | "All devices working" / "1 device has a problem"  | Per-category status summary with counts                              |
| **Network adapters**    | "Wi-Fi: Working" / "Ethernet: Disabled"            | Adapter name, driver version (Win), module (Linux), status, errors   |
| **Bluetooth**           | "Bluetooth: On and working" / "Not found"          | Adapter name, driver/module, firmware version, status                |
| **Audio**               | "Speakers: Working" / "Microphone: Not detected"   | Device name, driver/module, sample rate, status, default device      |
| **Input devices**       | "Keyboard: Working" / "Mouse: Working"             | Device name, driver/module, type (USB/BT/PS2), status               |

#### Platform-Specific Behavior

##### Windows — Full Driver Dashboard

Windows has the richest driver model and this section should take full advantage of it. Query via WMI (`Win32_PnPEntity`, `Win32_PnPSignedDriver`, `Win32_SoundDevice`, `Win32_Keyboard`, `Win32_PointingDevice`, `Win32_NetworkAdapter`) and/or the Setup API.

**For each device category, show:**
- Device name and hardware ID.
- Driver name, version, date, publisher, and signing status.
- Device status: **Working**, **Disabled**, **Error** (with problem code), **Not started**, **Driver missing**.
- In User Mode, translate problem codes into plain language:
  - Code 22 → "This device is turned off (disabled)"
  - Code 28 → "No driver installed for this device"
  - Code 31 → "This device isn't working properly"
  - Code 10 → "This device cannot start"
- In Technician Mode, show raw problem codes alongside descriptions.

**Network-specific on Windows:**
- List all network adapters (physical and virtual), with driver version and status.
- Show the status of critical network services: DHCP Client, DNS Client, WLAN AutoConfig, Network Location Awareness, WWAN AutoConfig (for cellular).
- Flag disabled adapters — this is an extremely common cause of connectivity issues.

**Bluetooth-specific on Windows:**
- Bluetooth adapter name, driver version, status.
- Bluetooth Support Service (`bthserv`) — running/stopped.
- Bluetooth User Support Service — running/stopped.

**Audio-specific on Windows:**
- List all audio endpoints (speakers, headphones, microphones).
- Windows Audio service (`Audiosrv`) — running/stopped.
- Windows Audio Endpoint Builder service — running/stopped.
- Driver info for each audio device.

**Input-specific on Windows:**
- List keyboards and pointing devices with driver info.
- HID (Human Interface Device) service — running/stopped.
- Flag any devices in error state.

##### macOS — Device & Service Health

macOS does not use installable drivers in the Windows sense. Most hardware is managed by built-in kernel extensions (kexts) or DriverKit system extensions. The section should adapt its framing accordingly.

**Network:**
- Network service ordering and active/inactive services (System Configuration framework, `networksetup`).
- Wi-Fi power state (CoreWLAN).
- Whether the Wi-Fi interface is present and recognized.
- Active VPN configurations.
- Firewall status (Application Firewall on/off, stealth mode).
- Loaded networking system extensions (relevant for third-party VPN/firewall products like Little Snitch, Mullvad, etc.).

**Bluetooth:**
- Whether the Bluetooth controller is present and powered on (IOBluetooth framework).
- Bluetooth firmware version.
- Number of paired/connected devices.
- `blued` daemon status.

**Audio:**
- Core Audio device list (output and input devices).
- Default output and input device.
- Whether audio devices are responding (sample rate, channel count).
- `coreaudiod` daemon status — a common troubleshooting step on macOS is restarting this daemon.

**Input:**
- List detected keyboards and pointing devices (via IOKit HID).
- Whether input devices are responding.
- For Apple trackpads: whether the multitouch driver is loaded.
- Accessibility permission status for input monitoring (relevant if input devices seem unresponsive to certain apps).

**In User Mode on macOS:** Frame everything as device health, not "drivers." Say "Wi-Fi adapter: Working" not "Wi-Fi driver: OK."

##### Linux — Kernel Modules & Services

Linux uses kernel modules for hardware drivers. This section should inspect module load status and relevant userspace services.

**Network:**
- For each detected network adapter (from PCI/USB enumeration): which kernel module is handling it (e.g., `iwlwifi`, `ath9k`, `e1000e`, `r8169`).
- Whether the expected module is loaded. If an adapter is detected on the bus but has no loaded module → **Driver missing**.
- NetworkManager / systemd-networkd / connman — which is in use and its status.
- wpa_supplicant status (for Wi-Fi authentication).
- `rfkill` status — whether wireless is soft-blocked or hard-blocked. This is a very common Linux Wi-Fi issue.
- Optionally: whether firmware blobs are present for adapters that need them (e.g., `iwlwifi` firmware files in `/lib/firmware`).

**Bluetooth:**
- Bluetooth adapter presence and kernel module (e.g., `btusb`, `btintel`, `btrtl`).
- `bluetooth.service` (BlueZ) — running/stopped.
- `rfkill` Bluetooth block status.
- Whether the adapter is recognized by BlueZ (`bluetoothctl` equivalent).

**Audio:**
- ALSA kernel module status (`snd_hda_intel`, `snd_usb_audio`, etc.).
- PulseAudio / PipeWire — which is in use and its status.
- Whether ALSA detects sound cards (`/proc/asound/cards` equivalent).
- Default sink and source.

**Input:**
- Input devices from `/dev/input/` or `libinput`.
- Kernel modules for input (e.g., `usbhid`, `i2c_hid`, `atkbd`, `psmouse`, `hid_multitouch`).
- Whether `libinput` or `evdev` is handling each device.
- Any devices present on the bus without a loaded driver.

**In User Mode on Linux:** Same plain-language framing as macOS. "Wi-Fi adapter: Working" or "Audio: PipeWire running, speakers detected."

#### Conceptual Layout — User Mode (Section 9)

```
  DEVICE HEALTH
  ─────────────────────────────────────────────────

  NETWORK
  ✓ Wi-Fi adapter              Working — Connected
  ✓ Ethernet adapter           Working — Cable unplugged
  — VPN adapter                Disabled (this is normal if not using VPN)

  BLUETOOTH
  ✓ Bluetooth                  On and working — 3 devices paired

  AUDIO
  ✓ Speakers                   Working — Set as default
  ✓ Microphone                 Working
  ⚠ HDMI Audio                 Not detected — Monitor may not support audio

  KEYBOARD & MOUSE
  ✓ Keyboard                   Working
  ✓ Mouse                      Working
```

#### Conceptual Layout — Technician Mode (Section 9)

```
  DRIVERS & DEVICES — Windows 11                           14:32:07 CST
  ──────────────────────────────────────────────────────────────────────

  NETWORK ADAPTERS
  ┌──────────────────────────┬────────────────┬──────────┬─────────────┐
  │ Device                   │ Driver Version │ Status   │ Driver Date │
  ├──────────────────────────┼────────────────┼──────────┼─────────────┤
  │ Intel Wi-Fi 6E AX211    │ 23.50.0.4      │ ✓ OK     │ 2024-11-15  │
  │ Realtek PCIe GbE        │ 10.76.601.2025 │ ✓ OK     │ 2025-01-20  │
  │ TAP-Windows V9 (VPN)    │ 9.24.7.601     │ — Disabl │ 2023-06-01  │
  └──────────────────────────┴────────────────┴──────────┴─────────────┘
  Services: DHCP ✓  DNS ✓  WLAN AutoConfig ✓  NLA ✓

  BLUETOOTH
  ┌──────────────────────────┬────────────────┬──────────┐
  │ Intel AX211 Bluetooth    │ 23.50.0.4      │ ✓ OK     │
  └──────────────────────────┴────────────────┴──────────┘
  Services: Bluetooth Support ✓  BT User Support ✓

  AUDIO
  ┌──────────────────────────┬────────────────┬──────────┬─────────────┐
  │ Realtek HD Audio         │ 6.0.9561.1     │ ✓ OK     │ 2024-09-20  │
  │ NVIDIA HD Audio (HDMI)   │ 1.4.0.1        │ ✓ OK     │ 2024-11-01  │
  └──────────────────────────┴────────────────┴──────────┴─────────────┘
  Services: Windows Audio ✓  Audio Endpoint Builder ✓
  Default Output: Realtek HD Audio — Speakers
  Default Input:  Realtek HD Audio — Microphone

  INPUT DEVICES
  ┌──────────────────────────┬────────────────┬──────────┬─────────────┐
  │ HID Keyboard Device      │ 10.0.26100.1   │ ✓ OK     │ 2024-06-01  │
  │ HID-compliant mouse      │ 10.0.26100.1   │ ✓ OK     │ 2024-06-01  │
  └──────────────────────────┴────────────────┴──────────┴─────────────┘
  Services: Human Interface Device ✓
```

#### Data Collection Approach

- **On Windows:** WMI queries and the Windows Setup API. These are read-only, non-privileged operations. The `wmi` Rust crate can query `Win32_PnPEntity`, `Win32_PnPSignedDriver`, `Win32_NetworkAdapter`, `Win32_SoundDevice`, etc. Service status can be queried via the Service Control Manager API.
- **On macOS:** IOKit for hardware enumeration, System Configuration framework for network services, CoreWLAN for Wi-Fi, IOBluetooth for Bluetooth, Core Audio for audio devices. Most of these are available without elevated privileges.
- **On Linux:** Parse `/sys/class/net/`, `/proc/asound/`, `/sys/class/bluetooth/`, `/dev/input/`, and use `lspci`/`lsusb` equivalent enumeration. Check `systemctl` status for services. Read `rfkill` state from `/sys/class/rfkill/` or the rfkill netlink interface. Kernel module info from `/sys/module/` or `lsmod` equivalent.

**Important notes:**
- This section is **read-only**. SD300 never installs, updates, enables, or disables any driver or device. It only reports status.
- The data in this section is mostly **static** — it doesn't change every second like CPU load. Refresh every 10–30 seconds, or only on manual refresh (press `r` to refresh while in this section).
- If no issues are found (all devices working, all services running), the section should still show all detected devices — it's informational even when healthy.

---

## 7. Technical Implementation Notes

### 7.1 Rust Crates to Evaluate

- **`ratatui`** — **The TUI framework.** This is the core dependency. Provides widgets (blocks, tables, charts, gauges, sparklines, tabs, lists), layout system, terminal rendering. Actively maintained, large community, excellent documentation. This is non-negotiable for the project.
- **`crossterm`** — Terminal backend for `ratatui`. Provides cross-platform terminal input/output. (Alternative: `termion` for Unix-only, but `crossterm` supports Windows.)
- **`sysinfo`** — Cross-platform system information (CPU, memory, disks, networks, processes, temperatures). This will be the primary data source for most metrics. Already battle-tested and likely used by TR-300.
- **`nvml-wrapper`** — NVIDIA GPU monitoring via NVML. For GPU utilization, VRAM, temperature, fan speed on NVIDIA cards.
- **`tokio`** — Async runtime for non-blocking data collection. Important for keeping the UI responsive while polling system metrics.
- **`clap`** — CLI argument parsing for `--user`, `--tech`, `--help`, `--version`, etc.
- **`serde` + `serde_json`** — If we want a `--dump` flag that exports current state as JSON (nice to have).
- **`wmi`** (Windows only, `#[cfg(target_os = "windows")]`) — WMI queries for driver enumeration, device status, service state. Essential for Section 9 on Windows.
- **`windows` or `winapi`** (Windows only) — Low-level Windows API access for Service Control Manager queries, Setup API, and PnP device enumeration.
- **`objc2` / IOKit bindings** (macOS only) — Hardware enumeration via IOKit, CoreWLAN for Wi-Fi, IOBluetooth for Bluetooth, Core Audio for audio devices.
- **`nix`** (Linux/macOS) — Unix system call wrappers for ioctl, sysfs reads, rfkill queries, and other low-level operations.
- **`neli`** or **`netlink-sys`** (Linux only) — Netlink interface for rfkill state, nl80211 Wi-Fi queries, and kernel interface details.

### 7.2 Architecture Recommendations

**Separation of concerns:**

1. **Data Layer** — Modules that collect system information. One module per section (cpu.rs, memory.rs, disk.rs, gpu.rs, network.rs, processes.rs, thermals.rs, drivers.rs). Each module exposes a struct with all relevant data and a `refresh()` method. The `drivers.rs` module will have the most platform-specific code — use `#[cfg(target_os = "...")]` gating and a common trait interface to keep the platform implementations isolated.
2. **Presentation Layer** — Modules that render the TUI. One module per section, with variants for User Mode and Technician Mode. Takes data structs as input, produces `ratatui` widgets.
3. **App Layer** — Main event loop. Handles input, manages state (current section, current mode), ticks the data layer, redraws the presentation layer.

**Event loop pattern:**

```
loop {
    // 1. Check for user input (key press)
    // 2. Handle input (switch section, quit, etc.)
    // 3. If tick interval elapsed, refresh data
    // 4. Render current section with current mode
}
```

This is a standard `ratatui` pattern. The developer should reference `ratatui`'s examples and the `ratatui` book.

### 7.3 Cross-Platform Considerations

SD300 must be fully cross-platform (Windows, macOS, Linux). Every section must work on all three platforms, but the depth and nature of information will differ. The guiding principle: **run every diagnostic that makes sense on the current OS, silently skip what doesn't apply, and never show confusing placeholders for inapplicable features.**

#### General Rules

- Detect the host OS at compile time (`#[cfg(target_os = "...")]`) or runtime (`std::env::consts::OS`) and select the appropriate implementation.
- Use a trait-based abstraction for platform-specific data collection (e.g., `trait DriverScanner`, `trait AudioDeviceEnumerator`) with separate implementations per OS.
- Features that are OS-specific should be **additively included** — present when applicable, absent when not. Don't show "N/A" rows for Windows-only features on Linux. Just don't show those rows.
- If an entire section would be empty on a given platform (unlikely, but possible), show a brief explanation rather than a blank screen.

#### Per-Section Platform Notes

**Temperatures (Section 8):**
- Linux: Good support via `sysinfo` / hwmon.
- macOS: Limited — SMC access is restricted. Show what's available, note limitations.
- Windows: Varies by hardware. WMI `MSAcpi_ThermalZoneTemperature` provides some data. Degrade gracefully.

**GPU (Section 5):**
- NVIDIA: Works on all platforms via NVML (`nvml-wrapper` crate).
- AMD: ROCm-SMI on Linux, limited on other platforms.
- Intel integrated: `sysinfo` may expose basic info.
- Apple Silicon GPU: Limited metrics available through IOKit.
- On unsupported configurations, show basic info or "Detailed GPU metrics not available."

**Process names vs. app names (Section 7):**
- On macOS and Windows, process binary names often differ from user-visible application names. Map binary names to friendly names (e.g., `chrome.exe` → "Google Chrome"). A built-in mapping table for common apps is important for User Mode.
- On Linux, the binary name is usually the application name, but Flatpak/Snap process names may be obscured.

**SMART data (Section 4):**
- Requires elevated privileges on most platforms. If not available, show "Drive health data requires administrator/root access" rather than crashing or omitting the section.

**Drivers & Devices (Section 9) — This is the most platform-divergent section:**

| Capability                     | Windows                              | macOS                                 | Linux                                  |
|-------------------------------|--------------------------------------|---------------------------------------|----------------------------------------|
| **Driver version/date/status** | Full — WMI, Setup API                | Not applicable (no user-facing drivers) | Kernel module name and load status     |
| **Device problem codes**       | Yes — PnP Manager problem codes      | No                                    | No (but can detect missing modules)    |
| **Network services**           | DHCP, DNS, WLAN AutoConfig, NLA      | Firewall, VPN config, Wi-Fi power     | NetworkManager, wpa_supplicant, rfkill |
| **Bluetooth services**         | BT Support Service, BT User Support  | blued daemon                          | BlueZ (bluetooth.service), rfkill BT   |
| **Audio services**             | Windows Audio, Endpoint Builder      | coreaudiod                            | PulseAudio / PipeWire, ALSA           |
| **Input device enumeration**   | WMI (Win32_Keyboard, PointingDevice) | IOKit HID                             | /dev/input, libinput, evdev            |
| **Firmware detection**         | Via WMI driver metadata              | IOKit device properties               | /lib/firmware checks for wireless FW   |
| **rfkill (wireless blocks)**   | No                                   | No                                    | Yes — critical for diagnosing Wi-Fi    |
| **System extensions/kexts**    | No                                   | Yes — loaded networking extensions     | Kernel modules (lsmod equivalent)      |

**The developer should treat Section 9 as having three substantially different implementations behind a common interface.** The User Mode presentation should use consistent, platform-neutral language ("Wi-Fi adapter: Working"), while Technician Mode shows raw platform-specific details (driver versions on Windows, module names on Linux, framework status on macOS).

#### macOS-Specific Notes

- macOS restricts access to certain system information without explicit entitlements or elevated privileges. Design around these restrictions — prefer APIs that work without `sudo`.
- Apple Silicon Macs have different hardware enumeration paths than Intel Macs. Test on both architectures.
- Certain metrics (fan speed, specific temperature sensors) require SMC access that may not be available to unsigned binaries. Document this limitation.

#### Linux-Specific Notes

- Linux distributions vary widely in which services are running (NetworkManager vs. systemd-networkd vs. connman, PulseAudio vs. PipeWire, X11 vs. Wayland for input). SD300 should detect what's present and report on it, not assume a specific distribution's stack.
- Kernel module loading and hardware enumeration should work consistently across distributions since it's kernel-level.
- Some diagnostics (SMART data, certain temperature sensors) may require root. Document the `setcap` approach or `sudo` recommendation.

#### Windows-Specific Notes

- WMI queries are the primary data source for Section 9 and work without elevation.
- Windows has the richest driver information and service management — this platform should have the most detailed Section 9 output.
- Support both Windows 10 and Windows 11. Some WMI classes or service names may differ between versions — handle gracefully.

### 7.4 Terminal Size Handling

The TUI must handle varying terminal sizes gracefully. `ratatui` provides a constraint-based layout system. The developer should:

- Define a minimum terminal size (e.g., 80x24). If the terminal is smaller, show a message asking the user to resize.
- Use `ratatui`'s `Layout` with percentage and min/max constraints so panels scale with the terminal.
- Technician Mode will naturally need more space. If the terminal is small, consider hiding less-critical panels or allowing the user to scroll.

---

## 8. CLI Interface

| Command / Flag      | Description                                          |
|---------------------|------------------------------------------------------|
| `sd300`             | Launch with mode selection screen                    |
| `sd300 --user`      | Launch directly into User Mode                       |
| `sd300 --tech`      | Launch directly into Technician Mode                 |
| `sd300 --help`      | Show help text                                       |
| `sd300 --version`   | Show version                                         |

### 8.1 Keybindings (In-App)

| Key          | Action                                      |
|--------------|---------------------------------------------|
| `1`–`9`      | Switch to section                           |
| `q` / `Esc`  | Quit the application                        |
| `m`          | Return to mode selection screen             |
| `?`          | Show help overlay                           |
| `j` / `k`    | Scroll (in scrollable views, Technician)    |
| `c` / `m` / `p` / `n` | Sort processes (Technician, Section 7) |
| `r`          | Manual refresh (Section 9 — Drivers & Devices) |

---

## 9. Distribution & Installation

Consistent with the rest of the 300 Series:

- **GitHub Releases** with prebuilt binaries for Windows (x86_64), macOS (x86_64, aarch64), Linux (x86_64, aarch64).
- **Shell installer:** `curl ... | sh` one-liner for macOS/Linux.
- **Cargo install:** `cargo install sd-300`.
- **Single static binary, zero runtime dependencies.** (NVML/GPU features degrade gracefully if the GPU drivers aren't present.)

---

## 10. Branding & Visual Consistency

- Application header: `SD-300 SYSTEM DIAGNOSTIC` / `QUBETX DEVELOPER TOOLS`.
- Color palette consistent with TR-300: greens, blues, white on dark terminal backgrounds. Accent colors for alerts (yellow warn, red fail).
- Box-drawing and table styling should match TR-300's aesthetic, adapted for the TUI context.
- The landing page (sd300.emmetts.dev or sd300.qubetx.com) follows the same design template as tr300.emmetts.dev.

---

## 11. Summary of Deliverables

1. **Rust binary** — `sd300` — cross-platform, single static binary.
2. **GitHub repository** — under the QubeTX organization, with README, LICENSE (PolyForm Noncommercial), CI/CD for release builds.
3. **Shell installer script** — consistent with TR-300's installation method.
4. **Landing page** — product page matching the TR-300 site design.

---

## 12. Non-Goals (Out of Scope)

- SD300 is **not** a process manager. It does not kill, restart, or manage processes. It is read-only / diagnostic-only.
- SD300 is **not** a replacement for dedicated GPU monitoring tools (like GPU-Z or `nvidia-smi`). It shows what it can, and degrades gracefully.
- SD300 does **not** modify system settings, overclock, or change power profiles.
- SD300 does **not** require network access. It is a purely local system diagnostic. (ND-300 handles network diagnostics.)
- SD300 is **not** a logging or alerting tool. It does not write logs to disk or send notifications. (A `--dump` JSON export could be a stretch goal, but it's not a core feature.)
