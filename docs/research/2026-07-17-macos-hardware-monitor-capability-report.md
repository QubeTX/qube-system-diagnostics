# SD-300 macOS Hardware-Monitor Capability Report

- **Research date:** 2026-07-17
- **Host tested:** MacBook Pro 13-inch, M2, 2022 (`Mac14,7`)
- **Host OS:** macOS 26.3.1 (build 25D2128), Darwin 25.3.0, native arm64
- **Repository baseline:** SD-300 1.4.3 at pre-research commit `43b1360`
- **Purpose:** portable, direct-identifier-sanitized evidence for the Alienware/Windows implementation phase
- **Change scope:** research and documentation only; no collector, TUI, CLI, release, or dependency behavior was changed

---

## Executive verdict

macOS can support a much more capable SD-300 than the current implementation, but not through one crate, command, or API. The evidence supports a layered backend:

1. **Stable public, unprivileged APIs** can cover identity, P/E topology, CPU and memory utilization, memory pressure, process statistics, interfaces, route/path state, Metal device capabilities, displays, audio, power-source basics, and much of device enumeration.
2. **Unprivileged but undocumented/model-specific IOReport, IOHID, AppleSMC, and I/O Registry routes** can expose unusually rich Apple Silicon temperatures, fan RPM/limits, CPU/GPU frequency residency, energy, AGX utilization/memory counters, battery electrical data, and block-device counters. These are useful Technician Mode experiments, not universal contracts.
3. **Root-only Apple tooling** (`powermetrics`) can expose the deepest live CPU/GPU/ANE frequency, residency, power, thermal, task-energy, disk, battery, and network views. The whole TUI must never run as root; this tier should be optional and isolated.
4. **Private APIs are not synonymous with root-only APIs.** Direct read-only AppleSMC and IOReport calls both succeeded as the ordinary user on this machine. They are still unpublished, brittle, and inappropriate as guaranteed/App-Store-safe support without further qualification.
5. **Some desired metrics simply are not available through supported public APIs**, particularly global ANE/media-engine utilization. This macOS 26 host unexpectedly exposed detailed Apple internal-NVMe SMART fields through an undocumented `diskutil info -plist` dictionary, but that route is explicitly named “may vary/not guaranteed.” “Exhaustive” must mean exhaustive capability detection and honest unavailable states, not invented values.

The current macOS collector is far below that ceiling. It builds and returns a small shell-based subset, but it also turns missing data into positive claims: an absent Apple GPU reading becomes “Good,” missing fans become “Quiet / not detected,” a shared maximum CPU frequency is described as live per-core speed, and normal Unknown device states become alerts. The first major implementation step should therefore be an observation/provenance model, before adding more probes.

The user’s interface direction is sound: keep the **Rust core + CLI + Ratatui TUI canonical**, then add any GUI as a separate consumer. The “Vercel native SDK using Zig” is real: [Vercel Labs Native](https://github.com/vercel-labs/native) is a pre-1.0, Zig-heavy cross-platform native-window/custom-rendering SDK. It is promising for an experiment, but not mature or widget-native enough to become the collector architecture.

### What the Alienware agent can do immediately

- Design the cross-platform observation/provenance types and platform capability registry.
- Refactor collectors behind testable interfaces without changing Mac-specific semantics blindly.
- Implement fixture-driven plist/JSON parsers with sanitized Mac fixtures derived from the shapes in this report.
- Correct source-independent logic errors: Unknown handling, load-vs-health language, scheduling, redaction, timeout/output draining, and UI scrolling/filtering.
- Build the Rust-side snapshot/stream protocol for a future GUI.
- Add macOS code behind compile-time gates and cross-compile it where possible.

### What must wait for a Mac validation pass

- Runtime proof of CoreWLAN/TCC behavior in the final signed `.app` and Terminal binary.
- IOHID temperature-channel mapping on additional SoCs and form factors.
- AGX/IORegistry private-counter compatibility across OS and chip generations.
- Any root `powermetrics` bridge, fan/SMC route, Intel sensor path, Rosetta runtime, sleep/wake, lid, hot-plug, external display/GPU/storage, or battery-discharge test.
- Developer ID signing, entitlements, notarization, hardened runtime, and app-bundle packaging.

---

## How to read the evidence

This Mac was the primary source of truth for host behavior. Apple and framework documentation were used to explain contracts and identify what one machine cannot prove. No raw profiler dump was committed.

### Evidence labels

| Label | Meaning |
|---|---|
| **Observed-local** | Successfully or unsuccessfully measured on this exact Mac, OS, user, and permission context. |
| **Public-contract** | Documented by Apple as a public API or supported system interface. |
| **Undocumented-local** | Works here through private/opaque properties, but Apple does not publish a supported contract. |
| **Root-local** | Advertised or observed locally, but rejected for the ordinary user and requires superuser. |
| **External-hardware** | Physical fact established by a product/repair source, not by telemetry. |
| **Inferred** | Best explanation of multiple observations; not a directly named Apple contract. |
| **Unverified-other-Mac** | Requires another model, architecture, OS release, or distribution context. |

### Generalization rule

One successful local result proves that a signal is possible **here**. It does not prove that every supported Mac exposes the same service, label, property, unit, permission, or update cadence. Likewise, a failed derived metric does not prove that the hardware is absent when an independent source proves it exists.

### Access tiers for the future collector

| Tier | Name | Examples | Product treatment |
|---:|---|---|---|
| 0 | Public, unprivileged | Mach host APIs, `ProcessInfo`, Metal, CoreGraphics, CoreAudio, SystemConfiguration, Network framework, Disk Arbitration | Default live collector |
| 1 | Public but absent/nonproductive here | Legacy IOPM CPU-power/thermal-warning functions returning not-found on this M2 | Detect and fall back; never call it failure |
| 2 | Apple command, unprivileged | `system_profiler`, `diskutil`, `pmset`, `systemextensionsctl` | Background/on-demand; strict field allowlist |
| 3 | Unprivileged, undocumented | IOReport energy/residency, IOHID sensors, read-only AppleSMC fan keys, AGX `PerformanceStatistics`, AppleSmartBattery raw keys, I/O Registry counters | Experimental Technician Mode with source/confidence warning |
| 4 | Root Apple command | `powermetrics` | Explicit opt-in; narrow helper or one-shot capture, never root TUI |
| 5 | TCC/entitlement/system-extension/helper | Wi-Fi identity, camera/mic use, Endpoint Security, some network inspection | Request only for a user-facing feature that needs it |
| 6 | Other private/unproven routes | unmapped SMC/IOReport groups, CoreDisplay/DisplayServices, model-specific power rails | Optional and fail-soft; do not promise stability |
| 7 | Third-party dependency | `smartctl`, model-specific sensor libraries | Detect only; do not install silently |
| 8 | Different-Mac-only | Intel SMC sensors, multiple GPUs, Mac Pro PCIe, fanless Air | Fixture + physical fleet qualification |

---

## Privacy handling and durable-data rules

The local probing confirmed that raw macOS diagnostic output can leak much more than expected. This report intentionally omits all stable machine identifiers and personal network/device data.

It does retain a few useful aggregate lifecycle snapshots such as battery cycles and coarsened SSD usage. Those are not direct identifiers, but combinations of lifecycle counters can be quasi-identifying. This research artifact is therefore **not** a template for default user exports: production exports should round or omit lifecycle counters unless the user explicitly includes them.

### Confirmed sensitive surfaces

- Hardware profiler: serial number, hardware UUID, provisioning UDID.
- Bluetooth profiler: remembered and nearby device **names can be JSON object keys**; addresses and component serials appear as values.
- Wi-Fi profiler/CoreWLAN: SSID, BSSID, hardware address, nearby networks, country/location-adjacent state.
- Power profiler/I/O Registry: battery and charger serials, scheduled wake events, raw lifetime data.
- NVMe, displays, cameras, Thunderbolt, USB, APFS: serials, UUIDs, unique IDs, volume names, topology identifiers.
- Network/process views: IPs, MACs, VPN/service names, paths, account names, process names, socket endpoints.

Removing values but retaining arbitrary JSON keys is not a sufficient redaction strategy. Collection must use explicit field allowlists and rebuild a safe typed object.

### Recommended sensitivity classes

| Class | Examples | Default export |
|---|---|---|
| Public machine class | model identifier, SoC family, core counts, RAM size, OS version | Include |
| Ephemeral telemetry | utilization, temperature, pressure, rate, fan RPM | Include, timestamped |
| Local labels | interface name, volume label, process name, audio/display name | Redact or pseudonymize |
| Stable identifier | serial, UUID, MAC, BSSID, device unique ID | Omit |
| Location/social graph | SSID, nearby Wi-Fi/Bluetooth names, paired devices | Omit |
| Content-adjacent | process path, remote endpoint, scheduled event | Omit unless explicit technician opt-in |

The future `snapshot --export` path should default to safe data, show a preview of sensitive categories, and require explicit confirmation for any expanded capture.

---

## Tested host baseline

### Hardware and toolchain

| Field | Sanitized value | Source | Confidence |
|---|---:|---|---|
| Product | MacBook Pro 13-inch, M2, 2022 | Model identifier cross-check | High |
| Model identifier | `Mac14,7` | `sysctl` / allowlisted hardware profiler | High |
| SoC | Apple M2 | `sysctl` / profiler | High |
| Process architecture | arm64, native | `arch`; `sysctl.proc_translated=0` | High |
| Hypervisor support | Present | `kern.hv_support=1` | High |
| CPU | 8 cores: 4 Performance + 4 Efficiency | `hw.perflevel*` | High |
| GPU | 10-core Apple M2, Metal 4 reported | profiler + Metal API | High |
| Unified memory | 8 GiB installed | `hw.memsize` / `ProcessInfo` | High |
| Internal storage | approximately 500 GB Apple SSD over Apple Fabric | `diskutil` / NVMe profiler | High |
| Display | built-in Retina, 60 Hz | CoreGraphics / AppKit / profiler | High |
| Battery | 114 cycles; 88% maximum capacity; condition Good during capture | power profiler | High for snapshot |
| Rust | `rustc`/Cargo 1.95.0 | local toolchain | High |
| Swift | 6.3.3 | local toolchain | High |
| Xcode | 26.6, build 17F113 | local toolchain | High |
| Clang | 21 | local toolchain | High |
| Zig / `native` CLI | Not installed | executable lookup | High |
| CMake / Ninja / pkg-config | Not installed | executable lookup | High |

No missing GUI prerequisites were installed because this is a borrowed machine and the task is research-only.

### CPU topology and caches

The local `hw.perflevel*` tree exposed both cluster classes without root:

| Cluster | Logical cores | L1 data per core | L1 instruction per core | Shared L2 | Cores per L2 |
|---|---:|---:|---:|---:|---:|
| Performance | 4 | 128 KiB | 192 KiB | 16 MiB | 4 |
| Efficiency | 4 | 64 KiB | 128 KiB | 4 MiB | 4 |

The reported cache-line size was 128 bytes. This is exactly the kind of Apple Silicon topology a generic “8 identical cores” view loses.

### Capture conditions

The machine was under a heavy research/build workload. CPU use, temperature, memory pressure, swap, AGX utilization, and process counts are **work-session samples**, not idle baselines and not health verdicts. No synthetic stress test was run.

At one loaded snapshot:

- load average was roughly in the high 20s/low 30s;
- memory pressure reported 22% free and pressure level 2;
- approximately 13.4 GiB of a 14 GiB encrypted swap allocation was in use;
- the hottest temperature channel reached the high 70s in the final series and had reached about 83°C during an earlier, heavier interval;
- `ProcessInfo.thermalState` still reported `nominal` and `pmset` reported no thermal/performance warning.

This is direct evidence that **high load/high raw temperature does not entail failing hardware**. Thermal pressure and throttling state must be modeled separately from temperature and workload.

---

## Local capability summary

| Domain | Best local result | Access | Suggested cadence | Main caveat |
|---|---|---|---|---|
| Identity/topology | model, M2, P/E cores, caches, native/Rosetta state | Unprivileged | Startup/on-change | Do not export unique IDs |
| CPU utilization | aggregate/per-core utilization | Unprivileged | 1 s | Delta-based; load is not health |
| CPU frequency/power | deep sampler advertised | Root | Optional 1–5 s capture | `powermetrics`; estimated power and schema caveats |
| Memory | Mach/sysinfo totals; pressure, compression, swap | Unprivileged | 1 s | Apple unified memory semantics |
| Processes | process list, resource use | Unprivileged, per-process limits vary | 1–3 s | Names/paths/users sensitive |
| Temperatures | 33 channels through current `sysinfo`; 38 services through direct IOHID | Unprivileged, undocumented labels | 2–5 s | Opaque mapping; invalid-looking channels |
| Thermal pressure | nominal/fair/serious/critical | Public unprivileged | Event/1–5 s | Not the same as temperature |
| Fan hardware | one physical internal fan | External source + AppleSMC `FNum=1` | Static | Count/key shape varies by Mac |
| Fan RPM | actual/target/min/max/mode read successfully | Unprivileged, private AppleSMC | 1–5 s experimental | Read-only; key case/type varies by generation |
| Battery/adapter | charge, health, cycles, capacity, electrical data | Mixed public + raw IOKit | 2–30 s | Units/keys vary; serials sensitive |
| GPU static | Metal device/capabilities, 10 cores | Public unprivileged | Startup/on-change | Capability is not utilization |
| GPU dynamic | AGX utilization/memory counters worked | Undocumented unprivileged | Experimental 2–5 s | Private schema; validate plausibility |
| CPU/GPU/ANE energy/frequency | IOReport energy/residency worked; `powermetrics` also advertised | Private unprivileged + optional root | Optional 1–5 s | IOReport unpublished; feature-detect every symbol/channel |
| Media/ANE presence | registry engines visible | Undocumented/static | Startup | Presence is not usable global utilization |
| Storage topology | APFS/disk graph, protocol, SMART Verified, TRIM | Apple CLI unprivileged | 30–300 s/on-change | Parse plist, never localized text |
| Disk I/O | block-driver cumulative byte/op/time/error counters | Undocumented IOKit | 1–5 s delta | Map physical devices carefully |
| Apple SSD wear/temp | macOS 26 `diskutil` exposed NVMe life-used, spare, temperature, hours, cycles, shutdown, data, and error fields | Apple CLI, unprivileged, undocumented schema | On-demand/30–300 s | Dictionary says `MayVaryNotGuaranteed`; feature-probe every key and preserve raw limbs |
| Network counters | interfaces and byte deltas | Public unprivileged | 1 s | Classify virtual/AWDL/VPN correctly |
| Wi-Fi radio | RSSI, noise, rate, channel, PHY | CoreWLAN unprivileged | 2–10 s | SSID/BSSID may require Location/TCC |
| Sockets | TCP via current parser | Unprivileged | 3–10 s | PID association absent; privacy-sensitive |
| Bluetooth controller | state, chipset, firmware, counts | Profiler unprivileged | On-change/30–300 s | Raw output exposes device graph |
| USB/Thunderbolt/PCI/HID | registry/profiler inventories | Unprivileged | On-change/on-demand | Stable IDs must be redacted |
| Displays | logical/backing/physical modes, scale, EDR | Public unprivileged | On-change | “Resolution” has multiple meanings |
| Audio | device inventory and public CoreAudio capabilities possible | Public unprivileged | On-change | Stream test/mic use needs consent |
| Camera | presence/capability possible | AVFoundation | On-change/on-demand | Never activate silently |
| Security posture | SIP, FileVault, Gatekeeper, extensions | Apple tools unprivileged | On-demand | Some databases require FDA; inventory sensitive |

---

## Thermal sensors and fans

### What the existing dependency already sees

`sysinfo 0.39.1` on Apple Silicon uses `IOHIDEventSystemClient` temperature services and returns product labels. SD-300 therefore already receives 33 live channels transitively, but its PC-oriented label matcher ignores all of them for derived CPU/GPU temperature. A lower-level direct IOHID probe matched 38 services and read all of them in roughly 46–55 ms. Source inspection explains the exact 38-to-33 loss: `sysinfo` deduplicates solely by `Product` label, so six gas-gauge services with the same label collapse to one, and duplicate refreshes target the already stored first service. `Product` is presentation metadata, not sensor identity; the new backend must not repeat that deduplication.

The final 12-sample series below was captured at one-second cadence during ongoing development activity. Values are rounded to two decimals. The labels are preserved because they are model/schema evidence, not unique identifiers.

| Sensor label | Min °C | Mean °C | Max °C | Interpretation status |
|---|---:|---:|---:|---|
| `NAND CH0 temp` | 59.00 | 60.00 | 61.00 | Likely storage/NAND; label is descriptive |
| `PMU tcal` | 51.85 | 51.85 | 51.85 | Calibration/reference-like; do not treat as component |
| `PMU tdev1` | 54.40 | 55.05 | 55.69 | Opaque device channel |
| `PMU tdev2` | 66.03 | 67.84 | 69.21 | Opaque device channel |
| `PMU tdev3` | 49.18 | 49.43 | 49.72 | Opaque device channel |
| `PMU tdev4` | -1.47 | -1.36 | -1.27 | Physically suspect; flag invalid/unmapped |
| `PMU tdev5` | -1.47 | -1.34 | -1.27 | Physically suspect; flag invalid/unmapped |
| `PMU tdev6` | 45.61 | 45.93 | 46.13 | Opaque device channel |
| `PMU tdev7` | 63.33 | 64.42 | 66.06 | Opaque device channel |
| `PMU tdev8` | 59.22 | 59.85 | 60.60 | Opaque device channel |
| `PMU tdie1` | 70.70 | 74.15 | 78.17 | Die-family channel; exact block unknown |
| `PMU tdie2` | 67.77 | 70.07 | 72.00 | Die-family channel; exact block unknown |
| `PMU tdie3` | 69.18 | 71.36 | 73.51 | Die-family channel; exact block unknown |
| `PMU tdie4` | 67.23 | 69.68 | 71.89 | Die-family channel; exact block unknown |
| `PMU tdie5` | 68.10 | 71.34 | 74.38 | Die-family channel; exact block unknown |
| `PMU tdie6` | 68.31 | 71.54 | 75.90 | Die-family channel; exact block unknown |
| `PMU tdie7` | 68.64 | 70.34 | 71.78 | Die-family channel; exact block unknown |
| `PMU tdie8` | 66.36 | 69.59 | 72.00 | Die-family channel; exact block unknown |
| `PMU2 tcal` | 51.85 | 51.85 | 51.85 | Calibration/reference-like |
| `PMU2 tdev1` | 58.50 | 58.74 | 58.93 | Opaque device channel |
| `PMU2 tdev2` | 53.27 | 53.38 | 53.50 | Opaque device channel |
| `PMU2 tdev3` | 60.35 | 60.62 | 60.87 | Opaque device channel |
| `PMU2 tdev4` | 56.52 | 56.64 | 56.79 | Opaque device channel |
| `PMU2 tdev5` | 52.86 | 52.97 | 53.04 | Opaque device channel |
| `PMU2 tdie1` | 64.09 | 65.07 | 66.15 | Die-family channel; exact block unknown |
| `PMU2 tdie2` | 63.55 | 64.69 | 65.39 | Die-family channel; exact block unknown |
| `PMU2 tdie3` | 63.22 | 64.49 | 66.04 | Die-family channel; exact block unknown |
| `PMU2 tdie4` | 61.71 | 63.91 | 65.17 | Die-family channel; exact block unknown |
| `PMU2 tdie5` | 62.79 | 64.03 | 65.39 | Die-family channel; exact block unknown |
| `PMU2 tdie6` | 62.14 | 64.22 | 65.82 | Die-family channel; exact block unknown |
| `PMU2 tdie7` | 63.55 | 64.49 | 65.39 | Die-family channel; exact block unknown |
| `PMU2 tdie8` | 64.41 | 65.31 | 66.15 | Die-family channel; exact block unknown |
| `gas gauge battery` | 38.80 | 38.80 | 38.80 | Battery/gas-gauge temperature |

Earlier in the heavier build/probe window, the hottest die channel reached approximately 83.05°C and the NAND channel reached 61°C. These are observations, not thresholds. Apple does not publish a universal critical threshold for these opaque labels.

### Required sensor model

Each reading should carry:

- random per-run channel pseudonym plus raw label; never persist an I/O Registry ID as sensor identity;
- normalized class only when evidence supports it (`die`, `battery`, `storage`, `calibration`, `unknown`);
- current/min/max/mean and sample age;
- source (`IOHID/sysinfo`, SMC, `powermetrics`, third-party);
- validity state and reason;
- model/OS mapping version and confidence;
- unit and any source threshold;
- whether it is safe for User Mode.

Never choose the hottest opaque channel and simply call it “CPU temperature” without a model-tested mapping. A reasonable interim UI is “Apple die sensor range: 64–78°C (mapping experimental)” in Technician Mode, while User Mode relies on `ProcessInfo.thermalState` for the supported pressure verdict.

### Fan result

- **Physical hardware:** the model mapping identifies this host as the 13-inch M2 MacBook Pro. A [model-specific fan replacement guide](https://www.ifixit.com/Guide/MacBook+Pro+13-Inch+2022+(M2)+Fan+Replacement/157929) documents one internal, shrouded radial/centrifugal laptop-blower-style assembly; that mechanical classification comes from the guide photographs, not from software telemetry.
- **Local telemetry:** an ordinary-user, read-only AppleSMC connection returned `FNum=1`, actual speed approximately **2,985 RPM**, target approximately **2,998 RPM**, recommended minimum approximately **1,199 RPM**, recommended maximum approximately **7,199 RPM**, and mode `0` (automatic). No write/control operation was attempted.
- **Key details:** the uppercase `F0Md` key existed; lowercase `F0md` and `Ftst` returned not-found. Fan values used SMC type `flt ` and decoded as little-endian IEEE floats on this machine; `FNum` used `ui8 `.
- **ABI trap:** with the observed 80-byte structure, the key is at offset 0, `keyInfo.dataSize` at 28, result at 40, command byte at 42, and data at 48. Command 9 reads key information and command 5 reads bytes. Placing the size in `data32` instead of `keyInfo.dataSize` produced SMC status `0x89` even though IOKit itself returned success.
- **Support status:** this is strong local evidence and a viable experimental backend, but Apple publishes no SMC protocol contract. Feature-detect connection, keys, type, size, count, and casing; never hard-code this M2 layout as universal.
- **Still not established:** impeller/blade count, motor/vendor/electrical specification, acoustic state, tachometer accuracy/calibration, App Sandbox/App Store acceptance, behavior after sleep/wake, or compatibility on other SoCs.

The availability states must distinguish:

```text
NotPresent      # e.g. a fanless MacBook Air after model/API confirmation
Available + Some(0)    # fan exists and the source explicitly reports stopped
Available + Some(n)    # fan exists and reports n RPM
PrivilegeRequired
Unsupported
PermissionDenied
ParseFailed
Stale
```

Fan-control writes are outside the monitor’s scope and pose thermal/hardware risk. SD-300 should remain read-only. [macOS SMC fan research](https://github.com/agoodkind/macos-smc-fan) is a useful implementation cross-check, but local feature detection remains authoritative.

### Thermal pressure and warnings

`ProcessInfo.thermalState` returned `nominal`, Low Power Mode was off, and `pmset -g therm` reported no warning during the captured workload. Stable User Mode language should be based on nominal/fair/serious/critical pressure, not an invented temperature boundary. Raw sensors belong in Technician Mode with the mapping/validity caveats above.

Apple public references: [`ProcessInfo`](https://developer.apple.com/documentation/foundation/processinfo), [IOHID manager device enumeration](https://developer.apple.com/documentation/iokit/1438391-iohidmanagercopydevices), and legacy [`IOPMGetThermalWarningLevel`](https://developer.apple.com/documentation/iokit/1557103-iopmgetthermalwarninglevel). The legacy IOPM call returned `kIOReturnNotFound` here, which is a supported-fallback case rather than a failed machine.

---

## CPU, scheduling, frequency, and power

### Safe, stable tier

Use `sysctlbyname` and Mach host APIs for static capabilities and utilization deltas. Preserve P/E cluster topology instead of flattening all logical processors. Apple documents capability discovery through [`sysctlbyname`](https://developer.apple.com/documentation/kernel/1387446-sysctlbyname/determining_system_capabilities), and Mach exposes processor/host statistics through [`host_processor_info`](https://developer.apple.com/documentation/kernel/1502854-host_processor_info) and [`host_statistics64`](https://developer.apple.com/documentation/kernel/1502863-host_statistics64).

Useful fields include:

- logical/physical/active processor counts;
- `hw.perflevel*` cluster names, counts, and cache hierarchy where present;
- per-core utilization deltas (user/system/idle/nice where available);
- load averages and scheduler pressure as workload, not health;
- OS-reported thermal pressure and Low Power Mode;
- process architecture, hardware architecture, and Rosetta translation state.

### Current-frequency trap confirmed locally

The existing app reported `3504 MHz` for all eight cores. Local `sysinfo 0.39.1` source shows that its Apple Silicon path reads the final `pmgr` `voltage-states5-sram` entry and returns a shared maximum-like value. It is not eight independently measured live clocks. The TUI currently converts it into “Running at full speed” and per-core current-frequency rows; both claims are unsound.

Represent frequency with explicit scope and semantics:

```text
frequency.value_mhz
frequency.scope = package | cluster | core
frequency.kind = nominal | maximum | requested | measured_average | instantaneous | unknown
frequency.window_ms
frequency.source
```

If only maximum frequency is known, label it “reported maximum,” not “current.”

### Unprivileged IOReport tier

Private `/usr/lib/libIOReport.dylib` was callable as the ordinary user:

- `IOReportCopyAllChannels(0, 0)` completed in about 405 ms;
- the result described 7,923 channels across 119 groups, 548 subgroups, and 231 drivers;
- serialized, the raw plist was about 4.68 MB, far too large and sensitive for regular export;
- creating an Energy Model subscription took about 4 ms;
- one-second deltas worked for per-core/cluster energy, CPU and GPU performance-state residency, CPU/GPU/ANE/DRAM-related energy, GPU throttling/CLTM/PPM states, memory controller, NVMe, Wi-Fi, interrupt, and many other groups;
- under the deliberately busy session, `CPU Energy` increased by about 14,905 mJ over 1.008 seconds, or approximately 14.8 W. This is a live loaded reading, not an idle baseline or cross-machine benchmark.

The observed private call sequence was:

1. `IOReportCopyChannelsInGroup`.
2. `IOReportCreateSubscription`.
3. `IOReportCreateSamples` twice around a monotonic interval.
4. `IOReportCreateSamplesDelta`.
5. `IOReportSimpleGetIntegerValue` for counters, or `IOReportStateGetCount` / `IOReportStateGetNameForIndex` / `IOReportStateGetResidency` for state channels.

The SDK contains a `libIOReport.tbd` symbol list but no supported consumer header. Every symbol, group, channel, state count, unit, and mapping must therefore be feature-detected and versioned. [macmon](https://github.com/vladkens/macmon) is a useful practical implementation reference, not an Apple compatibility guarantee.

### Exact local DVFS state tables

`hw.cpufrequency` and per-perf-level frequency sysctls were empty or unknown. IODeviceTree `pmgr` tables supplied the state mappings used by IOReport:

| Engine/cluster | Observed states (MHz) |
|---|---|
| Efficiency cores | 600, 912, 1284, 1752, 2004, 2256, 2424 |
| Performance cores | 660, 924, 1188, 1452, 1704, 1968, 2208, 2400, 2568, 2724, 2868, 2988, 3096, 3204, 3324, 3408, 3504 |
| GPU | off, 444, 612, 808, 968, 1110, 1236, 1338, 1398 |

The complete decoded `(MHz, second_raw_word)` pairs were:

```text
E CPU: (600,790) (912,790) (1284,790) (1752,820) (2004,890)
       (2256,960) (2424,1030)
P CPU: (660,790) (924,790) (1188,790) (1452,790) (1704,800)
       (1968,830) (2208,855) (2400,880) (2568,910) (2724,945)
       (2868,980) (2988,1020) (3096,1050) (3204,1140)
       (3324,1140) (3408,1140) (3504,1140)
GPU:   (0,125) (444,640) (612,685) (808,720) (968,765)
       (1110,810) (1236,850) (1338,890) (1398,925)
```

The second word is deliberately named `second_raw_word`, not voltage: its active-state values look voltage-like, but the off-state value and undocumented schema prevent a universal mV claim. Preserve it only in experimental Technician data until model-specific calibration exists.

On this M2, CPU IOReport labels such as `V6P0` map by index against `voltage-states1-sram` for E cores and `voltage-states5-sram` for P cores; GPU `P1...` states map against `voltage-states9`. These names and table numbers are firmware/model details, not a portable contract.

IOReport residency used `24Mticks`, summing to about 24 million ticks per elapsed second in this probe. Store the reported unit and validate the sum/window instead of assuming nanoseconds. IOReport GPU-temperature channels existed but returned zero; the IOHID route supplied real temperature values. HID power services on vendor page `0xff08` also returned raw-looking values that were not calibrated, so they must not be labeled amperes/volts without a verified conversion.

### Root `powermetrics` tier

Local `powermetrics -h` advertised CPU power/frequency, cluster residency, interrupts, task energy, instructions/cycles, QoS, and other samplers; non-root execution failed with “must be run as root.” Its help also warns that power values are estimates, should not be compared across devices, and that extra fields/schema may change.

Recommended optional flow:

1. Default TUI remains unprivileged.
2. Technician explicitly starts a bounded privileged capture or authorizes a minimal signed helper.
3. Parse machine-readable plist, which is NUL-separated per sample.
4. Store sampled scope/window/source and never compare estimated watts across models as a health threshold.
5. Stop automatically and drop privilege/helper connection.

Do not parse the human-readable format, run the TUI with `sudo`, or make root telemetry a requirement for a healthy status.

---

## Memory and processes

### Memory model needed on Apple Silicon

Apple Silicon uses unified memory, so “GPU VRAM + system RAM” is not a valid additive model. Present:

- physical memory;
- used, available, inactive, wired, compressed, purgeable/cache estimates;
- memory-pressure level/trend;
- swap total/used and page-in/page-out/swap-in/swap-out rates;
- compressor size and compression/decompression activity;
- page size (16 KiB on this host);
- process footprint/RSS with clear definitions;
- Metal recommended working-set size separately, not as physical VRAM.

Use native Mach statistics for the fast path and calculate rates from monotonic cumulative counters. `vm_stat` and `memory_pressure` are valuable validation tools, not ideal one-second production subprocesses.

The loaded snapshot’s large encrypted swap allocation and pressure level demonstrate why the UI should show trend and pressure rather than call memory use over a generic percentage “bad.”

### Process model gaps

The current collector sorts by CPU and truncates to 100 entries before a later Memory Mode sort, so a high-memory/low-CPU process can disappear. It also reports zero total threads. On macOS, `sysinfo::Process::tasks()` returns `None`, so “sum the sysinfo task lists” is not a fix. A future collector should gather a complete lightweight index, rank separately per view, and use `libproc` for a coverage-labeled thread lower bound plus lazy selected-process enrichment.

Candidate fields:

- PID/PPID, user, executable display name, start time, state;
- CPU delta, memory footprint/RSS, threads, file/socket counts where allowed;
- disk I/O, wakeups/context switches, priority/QoS;
- translated/native architecture;
- energy/GPU/network only when a supported source and permission exists;
- permission and sensitivity metadata per field.

Process names, users, paths, arguments, and endpoints are sensitive. Safe exports should aggregate or pseudonymize them.

---

## GPU, ANE, and media engines

### Public Metal capability result

A throwaway Swift probe using public Metal APIs returned:

| Field | Observed-local value | Correct interpretation |
|---|---|---|
| Device | Apple M2 (`applegpu_g14g`) | Static GPU identity |
| Unified memory | `true` | GPU and CPU share physical memory |
| Recommended max working set | about 5.73 GB | Driver recommendation, not dedicated VRAM |
| Current allocated size | 64 KiB during probe | Allocation by this Metal client, not global GPU use |
| Max buffer length | 4 GiB | Per-resource capability |
| Families | Apple 1–8, Common 1–3, Mac 1–2, Metal 3/4 | Capability set on this OS/device |
| Apple family 9 | Not supported | Expected model-generation boundary |
| Counter sets | `timestamp` only | Can profile this app’s Metal work, not global GPU utilization |
| `supportsRaytracing` | `true` | API functionality; does **not** prove dedicated hardware RT units |

Relevant public contracts: [`MTLDevice`](https://developer.apple.com/documentation/metal/mtldevice), [`hasUnifiedMemory`](https://developer.apple.com/documentation/metal/mtldevice/hasunifiedmemory), [`recommendedMaxWorkingSetSize`](https://developer.apple.com/documentation/metal/mtldevice/recommendedmaxworkingsetsize), [`currentAllocatedSize`](https://developer.apple.com/documentation/metal/mtldevice/currentallocatedsize), and [GPU counters](https://developer.apple.com/documentation/metal/gpu-counters-and-counter-sample-buffers).

### Experimental AGX result

The unprivileged I/O Registry exposed `AGXAccelerator` `PerformanceStatistics` at one loaded instant:

- device, renderer, and tiler utilization: 72%;
- allocated system memory: approximately 2.54 GB;
- in-use system memory: approximately 847 MB;
- allocated parameter-buffer memory: approximately 67 MB;
- recovery count/time: zero;
- GPU core count: 10;
- `IOReportLegendPublic`: true.

This is excellent Technician Mode evidence but remains undocumented. Field presence, scale, timing, and meaning must be versioned and plausibility-tested. “IOReportLegendPublic” in a registry object does not make IOReport.framework a documented SDK contract.

### Root GPU/ANE tier

`powermetrics` advertises `gpu_power` and `ane_power` samplers on this host. These should be treated as root-only optional measurements with model/OS tests. Static Metal capability can never substitute for global load, frequency, residency, or watts.

### Engine inventory

The local registry contained services corresponding to:

- AGX GPU;
- Apple Neural Engine/load balancer;
- Apple hardware video decoder;
- two JPEG-related engines/nodes;
- Apple ProRes hardware;
- PMU temperature/power sensor infrastructure.

Presence establishes a hardware/driver service, not a public global-utilization API. VideoToolbox/Core ML can establish codec/model capabilities or measure work initiated by SD-300, but should not be advertised as a whole-system ANE/media monitor without evidence.

### Current defect

[`src/collectors/gpu.rs`](../../src/collectors/gpu.rs) invokes only `nvidia-smi`, so it reports no GPU on this Apple M2. [`src/ui/sections/overview.rs`](../../src/ui/sections/overview.rs) then turns unavailable GPU data into a Good/integrated result. This is an invalid inference and a first-wave correctness fix.

---

## Battery, charger, and power

### Sanitized local snapshot

| Metric | Observed-local | Notes |
|---|---:|---|
| Charge | 100% | Snapshot only |
| Condition | Good | Apple-reported condition |
| Cycle count | 114 | Safe aggregate; not a stable ID |
| Maximum capacity | 88% | Profiler’s health percentage |
| Raw current/max capacity | 4367 / 4420 | Private raw fields; units/model semantics need confirmation |
| Design capacity | 5103 | Private raw field |
| Nominal charge capacity | 4553 | Private `NominalChargeCapacity` field; capacity units/model semantics still require validation |
| Pack voltage | 12,552 mV | Consistent with three cell readings near 4.18 V each |
| Current | 0 at full charge | Snapshot; charging state matters |
| Battery temperature | raw value near 3109, consistent with about 37.8°C if interpreted as deci-Kelvin; 38.8–39.2°C virtual/gas-gauge signal | Encoding and multiple temperature concepts require validation |
| Adapter | profiler reported 65 W electrical rating while product label said 67 W | Keep rated/negotiated/name separate |
| Negotiated ceiling | about 20 V × 3.27 A | Raw adapter properties |
| Live system/adapter power | about 25.1 W / 24.5 W at one sample | Raw telemetry; units inferred, model-specific |

The I/O Registry also exposed lifetime extrema/accumulators and `PowerTelemetryData`. These were not copied because raw field semantics are undocumented and the object contains identifying data. Sentinel time-remaining values such as `65535` mean unavailable/unknown and must never be rendered as a duration.

### Recommended battery model

- present/replaceable flag where knowable;
- charge percent and charging/discharging/full state;
- time remaining with explicit unknown/sentinel handling;
- cycle count;
- design/full/current capacity and derived health percentage;
- Apple condition/failure flags;
- voltage/current/derived watts with units and sign convention;
- one or more temperature channels with source;
- power source, adapter rated vs negotiated vs current power;
- Low Power Mode;
- power assertions preventing sleep/display sleep;
- stale age and source/permission;
- serials/IDs always excluded from normal export.

Use the public [`IOPowerSources`](https://developer.apple.com/documentation/iokit/iopowersources_h) family for the supported baseline, then allowlisted I/O Registry/profiler fields for deeper Technician Mode. Power assertions are available through [`IOPMCopyAssertionsStatus`](https://developer.apple.com/documentation/iokit/1557072-iopmcopyassertionsstatus).

The current [`src/collectors/thermals.rs`](../../src/collectors/thermals.rs) returns no battery and Unknown power on all non-Windows platforms even though this Mac exposes substantial data.

---

## Storage, APFS, SMART, and I/O

### What was available without root

The internal device reported:

- Apple SSD model class and firmware/revision;
- approximately 500.28 billion bytes raw capacity;
- Apple Fabric protocol;
- internal, solid-state media;
- SMART status `Verified`;
- TRIM `Yes`;
- hardware AES capability;
- APFS physical-store/container/volume topology through plist output.

macOS 26.3.1 also returned a dictionary named exactly
`SMARTDeviceSpecificKeysMayVaryNotGuaranteed` from
`/usr/sbin/diskutil info -plist disk0`. This was a material late finding: no
third-party tool, root prompt, or raw-device access was required. Across two
samples during this work session it reported:

| Field | Sanitized observed-local result | Interpretation rule |
|---|---:|---|
| `AVAILABLE_SPARE` | 100 | normalized percent, not bytes |
| `AVAILABLE_SPARE_THRESHOLD` | 99 | normalized threshold percent |
| `PERCENTAGE_USED` | 4 | estimated life consumed, **not** 96% health |
| `TEMPERATURE` | 332–335 K, about 59–62°C | composite temperature; validate a plausible Kelvin range before converting |
| `MEDIA_ERRORS_0/_1` | 0 / 0 | low/high limbs; zero is a real counter only after schema/type validation |
| `NUM_ERROR_INFO_LOG_ENTRIES_0/_1` | 0 / 0 | low/high limbs |
| `POWER_ON_HOURS_0/_1` | about 3.36 thousand / 0 | lifecycle data, coarsened here for privacy |
| `POWER_CYCLES_0/_1` | about 220 / 0 | lifecycle data, coarsened here |
| `UNSAFE_SHUTDOWNS_0/_1` | about 14 / 0 | lifecycle data, coarsened here |
| `DATA_UNITS_READ_0/_1` | about 707 million / 0 | about 362 TB using the NVMe 512,000-byte unit |
| `DATA_UNITS_WRITTEN_0/_1` | about 252 million / 0 | about 129 TB using the NVMe 512,000-byte unit |
| `HOST_READ_COMMANDS_0/_1` | about 22.7 billion / 0 | lifecycle counter, not instantaneous IOPS |
| `HOST_WRITE_COMMANDS_0/_1` | about 4.7 billion / 0 | lifecycle counter, not instantaneous IOPS |
| `CONTROLLER_BUSY_TIME_0/_1` | 0 / 0 | likely unpopulated here; mark Suspect until corroborated |

The `_0`/`_1` naming is not documented by Apple. The values match a low/high
128-bit limb interpretation and the corresponding NVMe SMART field widths, so
retain both source integers and derive only through a versioned, explicitly
experimental normalizer:

```rust
fn join_u128_le_limbs(low: u64, high: u64) -> u128 {
    ((high as u128) << 64) | low as u128
}

fn nvme_data_unit_bytes(units: u128) -> Option<u128> {
    units.checked_mul(512_000)
}

fn kelvin_to_celsius(k: u64) -> Option<f64> {
    (200..=500).contains(&k).then_some(k as f64 - 273.15)
}
```

Do not collapse `PERCENTAGE_USED` into a generic health score. NVMe defines it
as estimated life consumed and allows vendor/model interpretation; preserve
values beyond 100 if the source type permits them. Likewise, do not infer that
an all-zero controller-busy field means the controller has never been busy when
other lifecycle counters prove substantial use.

The I/O Registry’s block-storage statistics exposed cumulative values. One primary-device snapshot had approximately 1.20 TB read, 605 GB written, 55.5 million read operations, 14.5 million write operations, and zero reported errors/retries. These are cumulative counters: the product should display deltas/rates and retain the raw counter timestamp, not call the totals “current speed.” Time-field units must follow the source contract or remain unknown.

### What remains unavailable or unproven

- a **supported Apple API contract** for those detailed internal-NVMe fields;
- proof that `_0/_1` and every unit remain stable across macOS/model families;
- a standard NVMe passthrough route usable by ordinary third-party code;
- device-specific error-log records rather than only the cumulative entry count;
- a second independent local source for controller-busy time.

`SMART Verified` is still only a coarse status and does not imply the private
dictionary exists. `AppleNANDStatus=Ready` is not a wear score. `smartctl` was
not installed and was not added to the borrowed Mac; even when installed,
Apple internal storage may not expose standard passthrough data. The new
dictionary materially improves Technician Mode on this exact host, but its own
name forbids promoting it to a universal contract.

### APFS model requirements

Represent a graph, not parallel vectors:

```text
physical device -> APFS physical store -> container -> volume -> role/mount
                                      \-> snapshots
```

Include filesystem role, encryption/sealed/read-only state, physical vs logical capacity, free/available/purgeable distinctions, and shared-container semantics. Never pair a mounted volume with a drive by vector index.

### Parsing rules

- Prefer `/usr/sbin/diskutil ... -plist` and structured profiler JSON/plist.
- Use absolute tool paths and `LC_ALL=C` only as a defensive fallback; never depend on localized English labels.
- Bound runtime/output, drain stdout and stderr concurrently, and version parser fixtures.
- Redact serial, UUID, volume label, mount path, and external-device identifiers.
- Refresh device topology on demand/hot-plug; calculate I/O from native cumulative counters at 1–5 seconds.

The current [`src/collectors/disk_health.rs`](../../src/collectors/disk_health.rs) runs `diskutil list -plist` but discards it, hard-codes `disk0`, parses localized text for only model/media type, and returns Unknown for rich fields already available here. [`src/ui/sections/disk.rs`](../../src/ui/sections/disk.rs) correlates APFS mounts and physical drives by vector position, which is not a valid topology rule.

Apple references: [Disk Arbitration](https://developer.apple.com/documentation/diskarbitration/diskarbitration-h), [`kIOBlockStorageDriverStatisticsKey`](https://developer.apple.com/documentation/iokit/kioblockstoragedriverstatisticskey), [ATA SMART library header](https://developer.apple.com/documentation/iokit/atasmartlib_h), and [`volumeAvailableCapacityForImportantUsage`](https://developer.apple.com/documentation/foundation/urlresourcevalues/volumeavailablecapacityforimportantusage).

---

## Network, Wi-Fi, sockets, and reachability

### Baseline interfaces and rates

The current collector enumerated 28 interfaces during one pass, including the physical `en0`, loopback, AWDL, bridge, and many `utun` interfaces. The large count is normal for a modern Mac with Apple services/VPN-style interfaces; “an interface exists” does not mean the Internet is connected.

The native fast path should combine:

- `getifaddrs` for addresses and interface flags;
- routing/SystemConfiguration data for service mapping and default route/DNS;
- `NWPathMonitor` for satisfied/unsatisfied/requires-connection and expensive/constrained/interface-type state;
- native cumulative counters for packet/byte/error/drop rates;
- CoreWLAN for Wi-Fi radio/link data;
- a separate, privacy-gated socket/process enrichment path.

Apple references: [Network path monitor](https://developer.apple.com/documentation/network/nwpathmonitor), [`NWPath`](https://developer.apple.com/documentation/network/nwpath), [`SCDynamicStore`](https://developer.apple.com/documentation/systemconfiguration/scdynamicstore-gb2), [SystemConfiguration schema definitions](https://developer.apple.com/documentation/systemconfiguration/scschemadefinitions), and [`SCNetworkReachability`](https://developer.apple.com/documentation/systemconfiguration/scnetworkreachability-g7d).

### Wi-Fi results and TCC behavior

The local adapter was a Broadcom-class controller supporting 802.11a/b/g/n/ac/ax. During different snapshots it was associated on 5 GHz channel 48 with 80 MHz width. Link rate varied substantially during the research session: profiler/CoreWLAN snapshots observed values from roughly 130/173 Mb/s to 702 Mb/s, with RSSI around -56 to -59 dBm and noise around -93 dBm. That variability is expected and proves that link rate is a timestamped negotiation metric, not a static adapter speed.

The full Wi-Fi profiler pass took roughly 7.1 seconds. A public CoreWLAN Swift probe was much more suitable for live data and returned:

- radio on and interface `en0`;
- RSSI, noise, transmit rate, channel/band/width, mode/security enums;
- cached scan-result count;
- hardware-address presence.

It returned no SSID/BSSID and unknown country code in the third-party probe, while Apple’s own `system_profiler` could expose network identity. Location authorization and process attribution can affect Wi-Fi identity. This must be retested in:

1. an unsigned Terminal binary;
2. a Developer ID-signed CLI;
3. an unsandboxed signed `.app`;
4. a sandboxed/App Store candidate, if that distribution is ever pursued.

SSID, BSSID, nearby-network names, hardware addresses, and scan results must be excluded from default diagnostics. CoreWLAN references: [framework](https://developer.apple.com/documentation/CoreWLAN) and [`CWInterface`](https://developer.apple.com/documentation/corewlan/cwinterface). Apple DTS discussions on [Wi-Fi information privacy](https://developer.apple.com/forums/thread/732431) and [CoreWLAN/Location behavior](https://developer.apple.com/forums/thread/759044) should be revisited against the deployment target.

### Active connectivity tests

The current collector always resolves a Google hostname and pings a fixed public IP. This creates undisclosed traffic and can report false failure on captive, ICMP-blocked, filtered, or enterprise networks.

The macOS timeout is also wrong: [`src/collectors/network_diag.rs`](../../src/collectors/network_diag.rs) passes `ping -W 3`, but macOS defines `-W` in milliseconds, so the intended three-second wait becomes roughly 3 ms. The app then presents subprocess wall time rather than parsed ICMP RTT.

Recommended behavior:

- default to passive route/path/link state;
- make active probes explicit, configurable, and disclosed;
- test gateway, DNS, HTTPS, and captive-portal state as separate observations;
- use a product-controlled endpoint only if the privacy/availability policy justifies it;
- report `ICMP blocked` or `probe inconclusive` separately from `offline`;
- parse protocol timing, not process launch time.

### Sockets and process attribution

The current macOS route returned TCP only, with 92 connections and 19 listeners in one loaded snapshot, and no PIDs. It omitted UDP and richer ownership. A complete implementation should be permission-aware and should avoid demanding Network Extension/Endpoint Security entitlement merely to decorate a table.

Socket fields are highly sensitive. User Mode should show aggregate listening/established counts and unexpected-exposure hints. Technician Mode can show redacted/local-only endpoints by default, with an explicit reveal action. Network Extension packet tunnels are not a general system-monitor shortcut; see Apple’s [packet-tunnel expected-use cases](https://developer.apple.com/documentation/technotes/tn3120-expected-use-cases-for-network-extension-packet-tunnel-providers).

### Command quirks confirmed

- `wdutil info` without `sudo` printed usage yet returned exit 0. Successful process exit is not sufficient; validate payload/schema.
- `system_profiler SPAirPortDataType` returned one real interface and one mostly-null pseudo-interface. Filter by meaningful identity/state, not array position.
- Interface names are not semantic: `en0` can be Wi-Fi here, while device order and service mappings vary.

---

## Bluetooth, USB, Thunderbolt, PCIe, and HID

### Bluetooth

The Bluetooth controller exposed chipset, firmware, PCIe transport, power state, service capabilities, connected count, and remembered count without root. Profiler latency varied from about 0.18 seconds to several seconds across passes, reinforcing that it belongs in an on-change/slow worker.

The current parser treats the existence of controller JSON as health Good and ignores controller state. It also checks obsolete/wrong launchd paths. On this OS the live services were verified with:

```text
launchctl print system/com.apple.bluetoothd
launchctl print system/com.apple.audio.coreaudiod
```

Raw Bluetooth JSON is among the most dangerous profiler outputs because remembered-device names can be object keys. Reconstruct controller-only records from an allowlist; do not serialize the original subtree. If a GUI needs Bluetooth access, add `NSBluetoothAlwaysUsageDescription` only for a real user-visible feature; Apple documents the key [here](https://developer.apple.com/documentation/BundleResources/Information-Property-List/NSBluetoothAlwaysUsageDescription).

### USB/Thunderbolt/PCIe

`system_profiler -listDataTypes` on this OS includes `SPUSBHostDataType`, not the commonly guessed legacy type. Static/on-demand inventory can include:

- bus/transport, vendor/product class;
- negotiated link speed and power/current budget where supplied;
- internal/external/removable status;
- port/location path using a redacted stable-within-run token;
- current driver, DriverKit/system extension, and state;
- hot-plug generation and last-seen time.

Do not include serials, Thunderbolt domain UUIDs, or raw location identifiers by default. Prefer IOKit service matching and notifications for product code; profiler is a slow validation/fallback path. Apple’s public starting points are [IOKit](https://developer.apple.com/documentation/iokit) and [`IOServiceGetMatchingServices`](https://developer.apple.com/documentation/iokit/1514494-ioservicegetmatchingservices).

### HID/input and other sensors

The current macOS collector always inserts a keyboard and trackpad as healthy, even on desktops. Actual enumeration should use IOHID device services and model capabilities. Keyboard/mouse/controller enumeration does not justify Accessibility or Input Monitoring permission.

Potentially discoverable, model-dependent inputs include:

- built-in/external keyboard, mouse, trackpad, game controllers;
- lid/clamshell state;
- ambient-light sensors;
- accelerometer/sudden-motion sensor on older models;
- Touch Bar/Touch ID presence as capability only;
- peripheral battery state where the device exposes it.

These require model-specific validation. A registry class name alone does not prove that a stable or meaningful reading is available.

---

## Displays, audio, and cameras

### Display terminology trap

Three different values appeared for the built-in panel:

| Concept | Observed value | Meaning |
|---|---:|---|
| Profiler panel label | `2560x1600Retina` | Marketing/native-family descriptor; not the active backing mode here |
| CoreGraphics backing pixels | 2880 × 1800 | Rendered pixel dimensions for the current HiDPI mode |
| Logical desktop | 1440 × 900 points | Coordinate-space dimensions shown to applications |

CoreGraphics additionally reported built-in/main/active/online, 60 Hz, no mirror, rotation 0, and physical size approximately 286.87 × 179.29 mm. AppKit reported a 2× backing scale and Extended Dynamic Range values: current maximum 1, potential maximum 2, reference maximum 0 in this snapshot.

Do not label a single field “resolution.” Model logical points, backing pixels, physical panel/native descriptor, scale, refresh, HDR/EDR, color profile, rotation, mirroring, and connection separately. Display serial/vendor/model IDs should be redacted. See Apple’s [Quartz Display Services](https://developer.apple.com/documentation/CoreGraphics/quartz-display-services).

### Audio

The public CoreAudio path can provide device list, alive/running state, default input/output/system roles, transport, channels, nominal/available sample rates, buffer size/latency, volume/mute where supported, and hot-plug notifications. The current profiler parser merely marks every name Good and does not establish liveness/default role.

Use public CoreAudio functions for live product code; Apple’s reference index is [Core Audio functions](https://developer.apple.com/documentation/coreaudio/core-audio-functions). Enumerating devices should not activate the microphone. A loopback/playback/recording test must be an explicit user action and must explain the permission request.

### Camera

Camera inventory can use AVFoundation for model, transport, position, formats, authorization status, and availability. Never initialize a capture session or turn on the camera as a background “health test.” Unique IDs and continuity-device names are sensitive. A test image belongs behind a visible, revocable user action.

---

## Security, permissions, Rosetta, and virtualization

### Local state observed

- System Integrity Protection: enabled.
- Gatekeeper assessments: enabled.
- FileVault: on.
- Boot mode: normal; secure virtual memory enabled.
- Two enabled system extensions were present.
- User/system TCC databases were not readable to this ordinary process.
- No camera, microphone, Location, Accessibility, Input Monitoring, or Full Disk Access prompt was triggered.

These are this borrowed machine’s state, not product requirements and not a verdict on its owner. Raw security-extension inventories can expose installed security/VPN products and should be opt-in.

### Permission design

Request the narrowest permission only when a feature visibly needs it:

- **No permission:** system identity, CPU/memory, temperatures through the observed private route, Metal capabilities, public display/audio inventory, basic network counters.
- **Location/TCC may apply:** SSID/BSSID and some Wi-Fi identity.
- **Camera/microphone:** only explicit capture tests.
- **Accessibility/Input Monitoring:** not required for hardware inventory; do not request casually.
- **Full Disk Access:** do not require for a general monitor. Report unavailable protected details honestly.
- **Endpoint Security entitlement/system extension:** only for a separately justified security product feature, not general process/socket monitoring.
- **Root/helper:** optional `powermetrics` or similarly privileged one-shot probes only.

Relevant Apple guidance: [App Sandbox](https://developer.apple.com/documentation/security/app-sandbox), [Hardened Runtime](https://developer.apple.com/documentation/security/hardened-runtime), [macOS privacy settings](https://support.apple.com/guide/mac-help/change-privacy-security-settings-on-mac-mchl211c911f/26/mac/26), [Endpoint Security](https://developer.apple.com/documentation/endpointsecurity), and [DriverKit entitlement requests](https://developer.apple.com/documentation/DriverKit/requesting-entitlements-for-driverkit-development).

### CLI vs `.app` attribution

An Apple-signed tool may see a field that an unsigned third-party process cannot. A Terminal-launched binary can also inherit a different TCC attribution/permission context from a signed GUI app. Every sensitive route must be tested in the final distribution form, not just in a Swift script.

### Rosetta and virtualization

This process was native arm64 (`sysctl.proc_translated=0`). `kern.hv_support=1` proves hardware virtualization capability, not that the host is a guest. A future identity record should separate:

- physical hardware architecture;
- current process architecture;
- Rosetta translation state;
- hypervisor-framework capability;
- guest/virtual-machine detection with source and confidence.

Apple’s [Rosetta environment guide](https://developer.apple.com/documentation/Apple-Silicon/about-the-rosetta-translation-environment) is the public reference. The x86_64 target compiled locally, but Intel/Rosetta runtime behavior was not requalified for this repository during this research-only change.

---

## Current SD-300 macOS implementation audit

The existing TUI is a useful shell. The following table records why its current macOS conclusions must not be treated as comprehensive hardware health.

| Area | Current behavior | Local contradiction / risk | Required direction |
|---|---|---|---|
| README | Calls macOS support “Full” and driver scanning “IOKit-based” | macOS driver collector only shells out; no macOS framework dependency | Say “baseline support”; link this qualification report |
| CPU frequency | Same `sysinfo` value shown per core as live speed | 3504 MHz was a shared maximum-like DVFS table endpoint | Add frequency scope/kind/source |
| CPU topology | Eight homogeneous rows | Host has 4 E + 4 P cores with different caches/states | Model clusters first-class |
| CPU health | Generic utilization threshold | High utilization is workload, not failing hardware | Separate workload/pressure/errors/health |
| GPU | Only `nvidia-smi` | 10-core Apple M2 GPU proved by Metal/profiler | Add Metal + optional AGX/IOReport backend |
| GPU overview | Missing data becomes Good/integrated | No telemetry does not entail either claim | Render explicit unavailable reason |
| Temperatures | 33 raw sensors, derived CPU/GPU empty | Apple labels do not match PC substrings | Preserve channels; add mapping/validity/confidence |
| Thermal UI | CPU threshold applied to every sensor; no scrolling | Calibration/NAND/battery/invalid channels differ; table clips | Typed sensors, filters, virtualized/scrollable table |
| Fans | Non-Windows list forced empty; User Mode says quiet | AppleSMC actually reported one fan at ~2985 RPM | Read-only feature-detected SMC; distinguish missing/zero |
| Battery/power | Non-Windows battery `None`, power Unknown | Rich battery/adapter data was unprivileged | Add IOPowerSources + curated deeper fields |
| Disk | Plist discarded; `disk0` hard-coded; text parsed | APFS graph + SMART/TRIM/firmware were available | Parse structured graph; enumerate physical stores |
| Disk mapping | Volumes paired to drives by vector index | APFS sharing breaks positional mapping | Explicit identifiers/edges |
| Network state | Any interface means Connected | Loopback/AWDL/VPN/utun inflate interface count | Model kind, link, service, route, path separately |
| Wi-Fi | Name heuristic misses `en0` | CoreWLAN proved Wi-Fi | Use service/interface framework mapping |
| Connectivity | Fixed DNS/ping; wrong timeout unit | Privacy traffic and false failures; `-W 3` is 3 ms | Passive first; opt-in structured tests |
| Connections | TCP only, no PID; endpoints exposed | Incomplete and sensitive | Permission-aware enrichment/redaction |
| Drivers | Network name implies health; any BT JSON Good; audio names Good | State/liveness not tested | Call these inventory observations, not health |
| Input | Keyboard/trackpad hard-coded Good | False on desktops/other configurations | Enumerate actual HID devices |
| Services | Wrong/obsolete launchd labels/domain | Both daemons ran despite app saying stopped | Query correct bootstrap domain/native APIs |
| Scan status | Completed macOS scan remains `NotScanned` | State machine is wrong | Return Success/Partial/Failed with per-source errors |
| Overview issues | Unknown counted as needing attention | Normal unsupported ports become alarms | Unknown is neutral/informational |
| Processes | CPU-top-100 truncation before memory sort; threads 0 | Memory view can omit real leaders | Complete index + per-view ranking |
| Commands | PATH resolution; pipes drained only after exit | Hijack risk and possible pipe deadlock on large output | Absolute paths, concurrent bounded drain |
| Scheduling | socket command synchronous every 3 s | Slow command can freeze draw loop | All I/O off render loop |
| Clock | Epoch modulo day | UTC-like clock in a local header | Use local timezone-aware time |

Key source locations:

- platform collector boundary: [`src/collectors/platform/mod.rs`](../../src/collectors/platform/mod.rs);
- snapshot and refresh scheduling: [`src/collectors/mod.rs`](../../src/collectors/mod.rs), [`src/app.rs`](../../src/app.rs);
- macOS device collector: [`src/collectors/drivers/platform/macos.rs`](../../src/collectors/drivers/platform/macos.rs);
- temperatures/battery/fans: [`src/collectors/thermals.rs`](../../src/collectors/thermals.rs);
- disk health: [`src/collectors/disk_health.rs`](../../src/collectors/disk_health.rs);
- network diagnostics: [`src/collectors/network_diag.rs`](../../src/collectors/network_diag.rs);
- command runner: [`src/collectors/command.rs`](../../src/collectors/command.rs);
- core enums/sections: [`src/types.rs`](../../src/types.rs).

### Live current-collector pass

A temporary path-dependent probe outside the repository exercised the current library. In one approximately 4.1-second complete pass:

| Phase | Approximate time |
|---|---:|
| Static refresh | <1 ms |
| Second fast refresh | 19 ms |
| Slow refresh | 92 ms |
| Connections | 116 ms |
| Connectivity | 207 ms |
| Disk health | 611 ms |
| Drivers | 1,350 ms |

Separate direct command timings varied significantly: Wi-Fi profiler around 7.1 seconds; display profiler around 1.9–4.4 seconds; hardware profiler around 0.75 seconds; Bluetooth/audio passes from subsecond to several seconds; `memory_pressure -Q` around 10 ms; `top -l 2 -s 1 -n 0` around 3.5 seconds. The design must schedule by worst-case cost and enforce budgets, not assume one benchmark is stable.

---

## Required observation and capability model

### Availability is data

Every field should be an observation rather than `Option<T>` plus guessed defaults. Availability, value validity, and freshness are orthogonal: a finite but physically suspect sensor still has a raw value; a stale observation can still show its last good value; and an unprimed delta has no value yet. The authoritative, compile-oriented Rust enum/struct—including `ProtocolMismatch`, safe reason/tool/contract IDs, experimental state, and nonserialized monotonic fields—is defined once in [Cross-platform observation types](#cross-platform-observation-types); do not create a second simplified wire schema from this conceptual section.

Additional distinctions needed:

- capability vs current state vs cumulative counter vs interval rate;
- physical component vs firmware sensor vs synthetic aggregate;
- load/pressure/temperature/error/wear/condition/health;
- source-native threshold vs product policy threshold;
- supported-public vs experimental-private;
- zero vs missing vs stopped vs fanless;
- current-client GPU allocation vs whole-system GPU memory;
- logical display points vs backing pixels vs panel descriptor.

### Capability registry

At startup and on hardware change, produce a machine-readable registry:

```text
domain.metric
  detected: true/false
  source candidates: [...]
  selected source
  access tier
  supported scope/unit
  expected cadence/cost
  last error/reason
  model/os/arch qualifiers
  sensitive fields
```

Technician Mode should expose this registry. It turns “why is fan missing?” into a debuggable answer instead of a blank panel.

### Health reasoning

Do not run all percentages through one 75/90 threshold. A useful health verdict requires a claim-specific rule:

- CPU/GPU utilization: workload indicator, not hardware health.
- Memory pressure: OS pressure state and trend, not only percent used.
- Temperature: typed sensor + model/source threshold or thermal-pressure corroboration.
- Disk capacity: fullness concern, separate from SMART/wear/errors.
- Battery: Apple condition, capacity health, cycles, failure flags, temperature/electrical anomalies.
- Network: link/path/probe evidence, not interface count.
- Device: present/driver active/error state, not name existence.

Every derived verdict should retain the evidence IDs and explain the rule in Technician Mode.

---

## Collector architecture and cadence

### Recommended layering

```text
Platform-neutral Rust domain model
├── Public macOS backend
│   ├── Mach/sysctl/ProcessInfo
│   ├── IOPowerSources, Disk Arbitration, IOKit
│   ├── Metal/CoreGraphics/CoreAudio/AVFoundation
│   └── Network/SystemConfiguration/CoreWLAN
├── Enhanced macOS backend (feature-detected, experimental)
│   ├── IOHID temperature services
│   ├── AppleSMC read-only fan keys
│   ├── IOReport energy/residency/bandwidth
│   ├── IODeviceTree DVFS tables
│   └── allowlisted registry statistics
├── Optional privileged helper
│   └── bounded powermetrics plist stream
├── Command fallback/on-demand diagnostics
│   └── system_profiler, diskutil, pmset, systemextensionsctl
└── Consumers
    ├── CLI
    ├── Ratatui TUI (primary)
    ├── redacted snapshot/stream
    └── experimental native GUI
```

### Cadence budget

| Cadence | Suitable work | Forbidden work |
|---|---|---|
| Fast, ~1 s | Mach utilization/memory deltas, native interface counters, history updates | `system_profiler`, `diskutil`, `netstat`, full IOReport inventory |
| Medium, 2–5 s | temperatures, read-only SMC fans, selected IOReport subscriptions, battery electrical subset, selected disk counters, Wi-Fi radio | Re-enumerating all devices/channels each tick |
| Slow, 30–300 s | battery health, volume space, SMART summary, security state, fallback device status | blocking render/event loop |
| Event/on-change | display/audio/network/device topology, power source, sleep/wake | fixed polling when notifications exist |
| On-demand | full profiler, APFS graph, root `powermetrics`, system extensions, detailed privacy-sensitive scans | automatic startup capture/export |

Create private subscriptions/handles once and sample deltas; do not reopen/re-enumerate IOReport, IOHID, SMC, or CoreWLAN every tick.

### External-command safety contract

- absolute paths to Apple tools;
- known-clean environment and locale;
- no shell interpolation;
- monotonic timeout plus graceful terminate/kill;
- bounded stdout/stderr drained concurrently;
- maximum record count and byte size;
- structured format required where available;
- payload validation independent of exit code;
- cancellation on shutdown;
- background worker, never render thread;
- source/version/timing/error metadata retained;
- redaction before logging.

### Privileged-helper boundary

Do not elevate the application. If root telemetry is pursued, use a small signed helper with:

- fixed allowlisted operations and arguments;
- no arbitrary command/path execution;
- local authenticated IPC;
- version/ABI negotiation;
- bounded samples/output;
- immediate privilege drop or one-shot exit;
- audit record visible to the user;
- no fan writes, kernel extensions, or persistence unless separately designed/reviewed.

Apple’s modern helper direction should be reviewed through [Service Management](https://developer.apple.com/documentation/servicemanagement), signing, hardened runtime, and notarization guidance.

---

## Implementation-ready Rust blueprint

This section is intentionally prescriptive. It is the handoff contract for writing the backend on Windows and validating it later on macOS. It describes code boundaries and call sequences, not merely API names.

### Proposed source layout

```text
src/
├── observations/
│   ├── mod.rs                 # Availability, Observation, SourceId, Unit, Scope
│   ├── capability.rs          # Capability registry and selected-source logic
│   ├── sensitivity.rs         # export/redaction policy
│   └── health.rs              # evidence-backed derived verdicts
├── collectors/
│   ├── platform/
│   │   ├── mod.rs             # cfg dispatch + PlatformBackend trait
│   │   └── macos/
│   │       ├── mod.rs         # MacBackend composition; no raw unsafe calls
│   │       ├── service.rs     # long-lived worker/run-loop and schedules
│   │       ├── capabilities.rs
│   │       ├── identity.rs
│   │       ├── cpu.rs
│   │       ├── memory.rs
│   │       ├── processes.rs
│   │       ├── thermal.rs
│   │       ├── power.rs
│   │       ├── gpu.rs
│   │       ├── storage.rs
│   │       ├── network.rs
│   │       ├── devices.rs
│   │       ├── display.rs
│   │       ├── audio.rs
│   │       ├── camera.rs
│   │       ├── security.rs
│   │       ├── commands.rs    # absolute-path on-demand fallbacks only
│   │       └── ffi/
│   │           ├── mod.rs
│   │           ├── ownership.rs
│   │           ├── mach.rs
│   │           ├── iokit.rs
│   │           ├── iopower.rs
│   │           ├── iohid.rs
│   │           ├── smc.rs
│   │           ├── ioreport.rs
│   │           └── libproc.rs
│   └── fixtures/              # platform-neutral sanitized parser fixtures
└── export/
    ├── snapshot.rs
    └── redact.rs
```

Keep `unsafe` inside `macos/ffi`; upper modules receive owned Rust values or typed errors. Do not place macOS branches back into each generic collector ad hoc.

### Cargo and compile-gating plan

Use one coherent Apple binding family: [`objc2`](https://github.com/madsmtm/objc2). Move `serde` and `plist` to ordinary dependencies because cross-platform DTOs/parsers/fixtures need them. Add live Apple bindings only under a target table so Windows/Linux products do not link frameworks:

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
plist = "1"
tokio = { version = "1", features = ["rt-multi-thread", "macros", "time", "sync", "process", "io-util"] }
sysinfo = { version = "0.39", default-features = false, features = ["system", "disk", "network", "user"] }

[target.'cfg(not(target_os = "macos"))'.dependencies]
# Preserve the cross-platform component backend where it does not defeat the
# Mac dynamic/private-API boundary.
sysinfo = { version = "0.39", default-features = false, features = ["component"] }

[target.'cfg(target_os = "macos")'.dependencies]
objc2 = "0.6"
dispatch2 = "0.3"
libloading = "0.8"
objc2-core-foundation = "0.3"
objc2-foundation = "0.3"
objc2-io-kit = "0.3"
objc2-metal = "0.3"
objc2-core-graphics = "0.3"
objc2-core-audio = "0.3"
objc2-core-wlan = "0.3"
objc2-system-configuration = "0.3"
objc2-disk-arbitration = "0.3"
objc2-av-foundation = "0.3"
```

The locally verified family versions are `objc2 0.6.4`, framework crates `0.3.2`, and `dispatch2 0.3.1`. Re-resolve/pin when implementation starts and select only the required generated features: `NSProcessInfo`; CoreFoundation array/data/dictionary/number/run-loop/string/URL types; IOKit `ps`, `pwr_mgt`, and graphics; `MTLDevice`; CoreGraphics direct-display/configuration; CoreAudio hardware/types; `CWChannel`/`CWInterface`/`CWWiFiClient`/CoreWLAN types; SystemConfiguration dynamic-store/schema features; and Disk Arbitration disk/session features. Add Security only when implementing signature validation. Keep `objc2-app-kit` out of the CLI/TUI collector dependency set; add it as an optional GUI-only feature if the main-thread AppKit broker below is implemented. The existing Tokio feature set is insufficient for the blueprint: `sync` is needed for `watch`/`mpsc`, `process` for async child control, and `io-util` for bounded concurrent pipe reads. Add `signal` only if the implementation actually uses `tokio::signal`; Unix process-group termination itself can use the existing `libc` dependency.

The explicit `sysinfo` feature split is a correctness requirement, not dependency tidying. `sysinfo 0.39.1`'s default `component` feature statically references the same private Apple-Silicon IOHID event-system symbols that the new adapter intentionally loads dynamically. Leaving defaults enabled means an absent symbol can still break startup before fail-soft probing runs. Once direct Mac temperature collection lands, cfg the current `Components` callers off on macOS and do not use `sysinfo-components` as a Mac fallback. For a public-only/App-Store build, also enable an application feature that forwards `sysinfo/apple-app-store` (which enables its sandbox mode) and compile all enhanced private adapters out. Its sandbox component implementation is empty and process visibility is sharply reduced, so that profile needs explicit capability results; merely compiling out SD-300's new private modules is not enough.

Do not mix objc2-owned `Retained`/`CFRetained` handles with the older `core-foundation`, `metal`, `core-graphics`, `coreaudio-sys`, or `system-configuration` wrapper families unless an adapter boundary and ownership review justify it. Use the repository's existing `cfg(unix)` `libc` dependency for Mach and `sysctl` rather than duplicating it or adding `mach2`. Private SMC/IOReport/IOHID symbols still require local declarations because the SDK does not provide supported headers/bindings for the complete routes used here.

Every module declaration and framework link must be target-gated:

```rust
#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" { /* declarations */ }
```

### Minimum macOS and runtime API availability

Make the deployment floor an explicit product contract. The current locally
built arm64 release binary reports `LC_BUILD_VERSION minos 11.0` (SDK 26.5).
The implementation proposal is therefore to set and test
`MACOSX_DEPLOYMENT_TARGET=11.0` for **both** Apple targets so arm64 and x86_64
do not accidentally promise different baselines. If supporting Intel macOS
older than 11 is a product requirement, define a separate legacy support
profile and fleet gate rather than inheriting Rust's target default silently.

Deployment target does not make newer Objective-C selectors safe. For example,
the local Foundation SDK marks `NSProcessInfo.isLowPowerModeEnabled` as macOS
12+. Generated `objc2` methods do not automatically perform Swift-style
`#available` checks. Before every selector/property newer than the configured
floor:

1. check the running `operatingSystemVersion` against the SDK availability;
2. also test the actual receiver with `respondsToSelector:` (or the class with
   `instancesRespondToSelector:` where appropriate);
3. send the generated method only inside that proven branch;
4. otherwise return `Unsupported { reason: UnsupportedOsVersion }` or
   `MissingSelector`, never `false`, zero, or a message send that can raise an
   unrecognized-selector exception.

Apply the same review to newer Metal, CoreWLAN, AVFoundation, CoreGraphics, and
IOKit properties. For weak/new C symbols, weak-link or resolve with `dlsym` and
null-check before calling. `#[cfg(target_os = "macos")]` is only a compile-time
platform gate and says nothing about runtime OS version. Record each metric's
minimum OS/selector/symbol in the capability registry, then run at least one
real/virtual test at the deployment floor plus current macOS. Apple references:
[`respondsToSelector:`](https://developer.apple.com/documentation/objectivec/nsobjectprotocol/responds%28to%3A%29?language=objc),
[`isLowPowerModeEnabled`](https://developer.apple.com/documentation/foundation/processinfo/islowpowermodeenabled),
and [Objective-C API availability](https://developer.apple.com/documentation/swift/marking-api-availability-in-objective-c).

The objc2 framework crates auto-link their public frameworks. Private IOReport must load symbols dynamically from `/usr/lib/libIOReport.dylib`; private IOHID event-system symbols should likewise be resolved dynamically from `/System/Library/Frameworks/IOKit.framework/IOKit`. Keep each `libloading::Library` alive until every object created by its functions has been released. Do not give an absent private symbol the power to prevent process startup.

Private IOReport/IOHID/SMC use is not suitable for a Mac App Store profile under Apple’s public-API rule. If App Store distribution is pursued, compile a public-only backend/profile and reserve the enhanced backend for separately distributed Developer ID/notarized diagnostics; see [App Review Guideline 2.5.1](https://developer.apple.com/app-store/review/guidelines/).

### Cross-platform observation types

Codify source and access at the type level before collector work:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasonCode {
    MissingField,
    WrongType,
    EmptySuccess,
    InvalidRange,
    InconsistentCounters,
    UnsupportedOsVersion,
    MissingSelector,
    MissingSymbol,
    AccessDenied,
    ProcessExited,
    SourceChanged,
    SleepWakeReset,
    UserCancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolId {
    SystemProfiler,
    Diskutil,
    Powermetrics,
    SmartctlSystem,
    SmartctlHomebrew,
    SmartctlAdministratorConfigured,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct SourceContractId(String); // private field

// Implement TryFrom<String> and From<SourceContractId> for String. TryFrom
// accepts only a bounded ASCII token such as [a-z0-9._-]{1,96}; never '/',
// whitespace, raw payload text, a local path, or a machine identifier.
// Constructors remain in one module, and serde deserialization goes through it.

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Availability {
    Available,
    NotPresent,
    Unsupported { reason: ReasonCode },
    PermissionDenied { reason: ReasonCode },
    PrivilegeRequired { helper: Option<ToolId> },
    ToolMissing { tool: ToolId },
    WarmingUp { expected_window_ms: u64 },
    TimedOut { after_ms: u64 },
    OutputTooLarge { limit_bytes: u64, observed_at_least_bytes: u64 },
    Cancelled { reason: ReasonCode },
    ProtocolMismatch { contract: SourceContractId, reason: ReasonCode },
    ParseFailed { schema: SourceContractId, reason: ReasonCode },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Validity {
    Valid,
    Suspect { reason: ReasonCode },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Freshness {
    Fresh,
    Stale {
        last_good_unix_ms: u64,
        #[serde(skip, default)]
        last_good_monotonic_ns: Option<u64>,
        reason: ReasonCode,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation<T> {
    pub value: Option<T>,
    pub availability: Availability,
    pub validity: Validity,
    pub freshness: Freshness,
    pub source: SourceId,
    pub captured_unix_ms: u64,
    #[serde(skip, default)]
    pub captured_monotonic_ns: Option<u64>,
    pub window_ms: Option<u64>,
    pub unit: Unit,
    pub scope: Scope,
    pub confidence: Confidence,
    pub access: AccessTier,
    pub sensitivity: Sensitivity,
    pub experimental: bool,
    pub model_mapping_version: Option<SourceContractId>,
}
```

Enforce constructor invariants: `Available` requires `Some(value)`; unavailable/error states require `None`; only `Freshness::Stale` carries a last-good value forward; and health derivation consumes only `Validity::Valid`. Serde's externally tagged representation makes unit variants lowercase strings (`"available"`) and data-bearing variants lowercase objects (for example `{"unsupported":{"reason":"wrong_type"}}`); pin fixture tests to that shape. A finite `-1.3°C` IOHID result is `Available + Some(raw) + Suspect`, visible in Technician Mode with a warning. A nonfinite value has no serializable number and becomes ParseFailed. `ProtocolMismatch` means the transport succeeded but a versioned source contract did not; it must not be conflated with permission or absence. Monotonic timestamps exist only for in-process age/rate calculations and are skipped during serialization because they have no meaning in another process or boot. Export Unix timestamps as metadata, and reconstruct/rebase monotonic baselines after fixture load, restart, or sleep. Define `SourceId` as an enum or the same kind of validated token—not an unrestricted string. The serializable type contains only reason enums, tool enums, validated contract/mapping/source IDs, and scalar metadata—never free-form parser errors or administrator paths. Keep richer `CollectorError { domain, operation, os_code, source, private_detail }` internal, map it to a safe reason code at the export boundary, and log no raw payload.

### Backend trait and snapshot deltas

Use a long-lived platform backend rather than stateless `collect()` functions:

```rust
pub trait PlatformBackend {
    fn capabilities(&self) -> &CapabilityRegistry;
    fn collect_static(&mut self) -> StaticDelta;
    fn collect_fast(&mut self, now: MonotonicInstant) -> FastDelta;
    fn collect_medium(&mut self, now: MonotonicInstant) -> MediumDelta;
    fn collect_slow(&mut self, now: MonotonicInstant) -> SlowDelta;
    fn request_on_demand(&mut self, request: DiagnosticRequest);
}
```

The backend object is deliberately **not** bounded by `Send`: on macOS, Objective-C/CoreWLAN/notification/private handles may be `!Send`. Construct, use, and drop `MacBackend` on one dedicated thread; only command and delta DTOs crossing its channels must be `Send`:

```text
App/Tokio thread --Command--> Mac collector thread + CFRunLoop/autorelease pool
App/Tokio thread <--watch/mpsc-- immutable typed SnapshotDelta values
```

The worker owns `CWWiFiClient`, IOHID client, SMC connection, IOReport subscription, notification port, CoreGraphics/CoreAudio callbacks, and counter baselines. It does **not** own AppKit objects. The App receives only Rust-owned scalars/strings/vectors. It calls `watch::Receiver::borrow_and_update()` during the draw loop—never a framework API or shell command.

At thread start:

1. create an autorelease pool;
2. probe stable capabilities;
3. dynamically load private sources and record why each failed;
4. create long-lived subscriptions/handles once;
5. publish static/capability deltas;
6. run scheduled sampling plus a CFRunLoop/dispatch notification source;
7. refresh the autorelease pool periodically;
8. on shutdown, unregister callbacks, release CF/I/O objects, close connections, then join.

Use monotonic time for deltas and wall time only for display/export. If a source blocks past budget, keep its prior value and set `Freshness::Stale`; do not block other sources.

### FFI ownership rules

Implement RAII wrappers and make ownership visible in function names:

- CoreFoundation functions containing `Create`/`Copy` return +1 objects: `OwnedCf<T>` calls `CFRelease` in `Drop`.
- `Get` functions return borrowed objects: never release them; convert/copy needed data before the owner dies.
- IOKit `io_object_t` values are released with `IOObjectRelease`.
- `io_connect_t` is closed with `IOServiceClose`, not only `IOObjectRelease`.
- Mach arrays returned out-of-line are released with `vm_deallocate`; for `host_processor_info`, convert its returned **integer-element count** to bytes with checked `count * size_of::<integer_t>()` rather than treating the count itself as bytes.
- Objective-C `Retained<T>` stays on its allowed thread; use `autoreleasepool` around sampling.
- Callback context pointers use one explicit `Arc::into_raw` registered reference. Each callback only borrows `&*(ptr as *const CallbackCtx)`; it must not call/drop `Arc::from_raw`, because doing so would consume the registered strong count and make later callbacks use freed memory. On shutdown, unregister, cancel/detach, and synchronously drain the framework queue/run loop; only then call `Arc::from_raw` exactly once to drop the registered reference.
- Never transmute CF/Objective-C types without checking `CFGetTypeID` or the framework binding’s type.

Suggested wrappers:

```rust
use std::{marker::PhantomData, rc::Rc};

struct IoObject {
    raw: io_object_t,
    _thread_bound: PhantomData<Rc<()>>,
}
struct IoIterator {
    raw: io_iterator_t,
    _thread_bound: PhantomData<Rc<()>>,
}
struct IoConnect {
    raw: io_connect_t,
    _thread_bound: PhantomData<Rc<()>>,
}
struct OwnedCf<T: CfRef> {
    raw: *const T,
    _thread_bound: PhantomData<Rc<()>>,
}
```

The private `PhantomData<Rc<()>>` is the stable way to prevent automatic `Send` and `Sync`; user-defined negative impls such as `impl !Send` remain unstable on this repository's Rust 1.95 toolchain. Construct wrappers only on their owner thread. A callback context is a separate minimal `Send + Sync` shim containing only an atomic shutdown flag and nonblocking sender—never the `!Send` backend or Apple handles:

```rust
struct CallbackCtx {
    shutting_down: AtomicBool,
    dirty_tx: Sender<DirtyEvent>,
}

extern "C" fn callback(raw: *mut c_void) {
    if raw.is_null() { return; }
    let ctx = unsafe { &*raw.cast::<CallbackCtx>() }; // borrow; no from_raw
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if !ctx.shutting_down.load(Ordering::Acquire) {
            let _ = ctx.dirty_tx.try_send(DirtyEvent::Refresh);
        }
    }));
}
```

Unit-test `Drop` through injected function tables; integration-test callback teardown and leak behavior on Mac.

### Capability/source selection algorithm

For each metric, register ordered candidates rather than hard-code one fallback chain:

```text
temperature.raw_channels: IOHID-direct -> unsupported on Mac
thermal.pressure: ProcessInfo -> pmset fallback
fan.rpm: AppleSMC-readonly -> optional third-party -> unsupported
cpu.frequency_residency: IOReport+DVFS -> root powermetrics -> maximum-only sysinfo
gpu.static: Metal -> allowlisted profiler
gpu.dynamic: IOReport -> AGX registry -> root powermetrics -> unsupported
battery.basic: IOPowerSources -> pmset
battery.deep: allowlisted AppleSmartBattery -> profiler
storage.graph: DiskArbitration + diskutil plist
storage.io: IOKit block counters
```

Candidate activation checks symbol/service/property presence, access, unit validation, and a plausibility sample. Store both the selected source and rejected candidate reasons. A source can be degraded independently per metric; e.g. IOReport energy may work while its GPU-temperature channel returns zero.

### `sysctlbyname` helper and identity/topology adapter

The raw function is stable and simple enough to wrap directly:

```rust
unsafe extern "C" {
    fn sysctlbyname(
        name: *const c_char,
        oldp: *mut c_void,
        oldlenp: *mut usize,
        newp: *mut c_void,
        newlen: usize,
    ) -> c_int;
}
```

Read-only algorithm:

1. Convert a constant key to `CString`; reject interior NUL.
2. Call with `oldp=null` to obtain length.
3. If error is `ENOENT`/`EINVAL`, return Unsupported; on `ENOMEM`, repeat the size query and retry once because a variable-length value changed; if length is zero, return Unsupported/empty, not zero.
4. Allocate exactly that many bytes and call again.
5. Decode known integer keys only when byte width matches; decode strings after trimming trailing NUL.
6. Never pass `newp`/`newlen` for a monitor.

Read static keys: `hw.model`, `hw.machine`, `hw.ncpu`, `hw.physicalcpu`, `hw.logicalcpu`, `hw.memsize`, `hw.cachelinesize`, `hw.nperflevels`, `kern.osrelease`, `kern.osversion`, `kern.boottime`, `kern.hv_support`, and `sysctl.proc_translated` (missing means unsupported; do not treat as translated). Read `vm.swapusage` with its declared struct/size, not text parsing.

Enumerate `hw.perflevel{n}` from `n=0` until a complete level is absent. For each, attempt `name`, `physicalcpu`, `logicalcpu`, `l1icachesize`, `l1dcachesize`, `l2cachesize`, `cpusperl2`. Preserve absent individual keys. Do not assume level 0 is always P or that there are only two levels; normalize names after capture.

Frequency sysctls require payload validation: this Mac returned successful-but-empty or unknown values. Never turn empty into 0 MHz.

### Mach CPU and memory adapter

Use `libc`'s `host_processor_info(mach_host_self(), PROCESSOR_CPU_LOAD_INFO, ...)` for per-core cumulative tick arrays. Retain the previous sample and calculate signed-tick values as wrapping `u32` deltas for user/system/nice/idle; if core count changes or the sample is internally inconsistent, discard the baseline and emit Stale for one interval. Release the returned out-of-line array with `vm_deallocate(mach_task_self(), ptr, info_count * size_of::<integer_t>())`; `info_count` is an integer count, not a processor count. Cache the `mach_host_self()` send right and call `mach_port_deallocate` on drop.

Use `host_statistics64(..., HOST_VM_INFO64, ...)` for VM counters and `host_page_size` for bytes. Derive byte fields with checked multiplication. Preserve counters for page-in/out, faults, copy-on-write, zero-fill, reactivation, purge, compression/decompression, swap-in/out. Name derived semantics explicitly; do not claim that `free_count * page_size` equals memory available.

Recommended memory observation set:

```text
free, active, inactive, speculative, wired, compressed, purgeable bytes
page faults/pageins/pageouts/cow/zero-fill counters and per-second deltas
compressions/decompressions/swapins/swapouts and deltas
swap total/used/free from sysctl or sysinfo
OS pressure state/percentage as its own source
```

The existing `sysinfo::System` can remain the first implementation for CPU/process/memory while direct Mach adapters are introduced behind parity tests. Do not instantiate two independently refreshed `System` handles per tick.

### Public memory-pressure event adapter

Do not derive Apple's memory-pressure state from `free / total`. Use a public
Grand Central Dispatch memory-pressure source. `dispatch2 0.3.1` exposes the
generated `_dispatch_source_type_memorypressure` symbol, `DispatchSource::new`,
`DispatchSource::data`, and the exact flag wrapper:

```rust
use dispatch2::{
    dispatch_source_memorypressure_flags_t::{
        DISPATCH_MEMORYPRESSURE_CRITICAL,
        DISPATCH_MEMORYPRESSURE_NORMAL,
        DISPATCH_MEMORYPRESSURE_WARN,
    },
    DispatchObject, DispatchQueue, DispatchSource,
    _dispatch_source_type_memorypressure,
};

const PRESSURE_MASK: usize =
    DISPATCH_MEMORYPRESSURE_NORMAL.0 as usize
    | DISPATCH_MEMORYPRESSURE_WARN.0 as usize
    | DISPATCH_MEMORYPRESSURE_CRITICAL.0 as usize;
```

On the Mac worker, create one serial queue and one inactive source with type
`unsafe { core::ptr::addr_of!(_dispatch_source_type_memorypressure).cast_mut() }`,
handle `0`, the combined mask above, and that queue. (`dispatch2` declares the
extern static immutable even though the C typedef is a mutable raw pointer;
`addr_of_mut!` does not compile on Rust 1.95.) Install an event handler before
calling `activate()`.
The handler must call `source.data()` **inside the handler**—the API says reading
pending data elsewhere is undefined—and reduce coalesced bits with strict
precedence `CRITICAL > WARN > NORMAL`. It should only publish a small
`MemoryPressureChanged` DTO; it must not allocate heavily, render, run commands,
or purge application caches blindly.

Do not strongly capture `DispatchRetained<DispatchSource>` inside its own event
block merely to call `data()`; that creates `source -> handler -> source` retain
cycle. Give the block a non-owning raw source pointer in a pinned callback
context, set an atomic shutdown flag before cancel, and keep the external owner
alive until the serial queue and cancel handler have drained.

The dispatch source is event-driven rather than an initial-state query. Until
the first event, report `Availability::WarmingUp`; do not initialize it from a
guessed free-memory threshold. On shutdown, cancel the already-activated source,
allow its serial queue/cancel handler to drain, then drop the retained source,
queue, block/context, and receiver in the documented ownership order. Never
dispose an unactivated dispatch source.

Two command observations are useful only as validation evidence: during this
loaded session `memory_pressure -Q` reported roughly 22–25% system-wide free
percentage, while the undocumented `kern.memorystatus_vm_pressure_level`
reported integer `2`. Neither is the production event contract, and the integer
must not be reverse-engineered into an enum. Apple's pressure notification is a
signal about how the process should adapt future memory use, not proof that a
particular free-byte percentage is good or bad.

### `ProcessInfo` adapter

Using `objc2-foundation`, keep the shared `NSProcessInfo` retained on the Mac thread. Sample processor count, active count, physical memory, system uptime, operating-system version, thermal state, and Low Power Mode. Register for thermal/low-power notifications if the binding exposes them; otherwise poll at medium cadence. Do not collect `hostName` or `globallyUniqueString`; they are unnecessary identifiers.

Map the enum exhaustively with an Unknown future case:

```text
0 nominal
1 fair
2 serious
3 critical
other unknown(raw)
```

Do not derive a Celsius threshold from this state. `systemUptime` may exclude sleep differently from wall-clock-since-boot; store its source semantics rather than relabel it boot age.

### IOHID temperature adapter

Resolve the private symbols dynamically from the IOKit framework binary and store these exact function-pointer types:

```rust
#[repr(C)] struct __IOHIDEventSystemClient { _private: [u8; 0] }
#[repr(C)] struct __IOHIDServiceClient { _private: [u8; 0] }
#[repr(C)] struct __IOHIDEvent { _private: [u8; 0] }

type IOHIDEventSystemClientRef = *const __IOHIDEventSystemClient;
type IOHIDServiceClientRef = *const __IOHIDServiceClient;
type IOHIDEventRef = *const __IOHIDEvent;

type EventSystemClientCreateFn =
    unsafe extern "C" fn(CFAllocatorRef) -> IOHIDEventSystemClientRef;
type EventSystemClientSetMatchingFn =
    unsafe extern "C" fn(IOHIDEventSystemClientRef, CFDictionaryRef); // ignore any register residue
type EventSystemClientCopyServicesFn =
    unsafe extern "C" fn(IOHIDEventSystemClientRef) -> CFArrayRef;
type ServiceClientCopyPropertyFn =
    unsafe extern "C" fn(IOHIDServiceClientRef, CFStringRef) -> CFTypeRef;
type ServiceClientCopyEventFn = unsafe extern "C" fn(
    IOHIDServiceClientRef, i64, i32, i64,
) -> IOHIDEventRef;
type EventGetFloatValueFn =
    unsafe extern "C" fn(IOHIDEventRef, i64) -> f64;
```

Initialization:

1. Create the client with `kCFAllocatorDefault`; null means Unsupported.
2. Build and retain a CFDictionary with type callbacks, `PrimaryUsagePage=0xff00`, and `PrimaryUsage=5` using CFNumber values. Release temporary key/number objects after insertion, but keep the dictionary through the client lifetime.
3. Call `SetMatching` and ignore the return register. Current `sysinfo` and macmon declarations type this private symbol as `i32`, but neither uses the value. Direct macOS 26.3.1 arm64 disassembly showed no defined return-value setup, and a local `ctypes.c_int` declaration produced meaningless register residue (`10084464`) even though 38 services were matched. Declaring it as returning nothing is therefore the conservative wrapper ABI: it safely ignores a value on an older implementation that might return one and does not invent a status on this implementation. Validate the result by copying and inspecting the matched service set.
4. Copy services once; refresh on wake/hardware change or if reads repeatedly fail. A separately tested public `IOHIDEventSystemClientCreateSimpleClient` enumerated the same 38 matched services but every temperature `CopyEvent` returned null; the private `IOHIDEventSystemClientCreate` route was required for values on this host.
5. For each service, copy the `Product` property, verify its CF type is string, and convert with a bounded dynamically sized UTF-8 buffer. Never use arbitrary product labels as stable cross-machine identities.

Sampling:

1. Event type is `15` (`kIOHIDEventTypeTemperature`).
2. Call `CopyEvent(service, 15, 0, 0)`; null means unavailable for that service.
3. Read field `15 << 16` (`983040`) with `IOHIDEventGetFloatValue`.
4. Release the copied event.
5. Reject nonfinite values as ParseFailed. Preserve finite values `<=0` or `>150°C` as `Validity::Suspect { reason }` with the raw value in Technician Mode, but exclude them from mapped aggregates and health logic. This retains the locally important `-1.3°C` evidence without pretending it is a physical component temperature.
6. Publish raw-label channels and model-mapped aggregates separately.

Ownership: the event-system client, copied services array, copied Product property, and copied event are independently owned CF objects and must each be released once. Service pointers obtained from the array are borrowed and must not be released individually.

The full 38-service sweep took 46–55 ms here, so sample at 2–5 seconds, not every draw. Six battery-gauge services shared a Product name, so use a per-sample ordinal only for display and never persist registry IDs. Do not copy Chromium’s older `pACC/eACC` label assumptions as universal—the current host used PMU labels. Keep a mapping table keyed by model-family + OS range and show mapping confidence. Vendor page `0xff08` usages also exposed raw-looking power values around 16,000; do not label those amperes/volts until calibrated.

### Read-only AppleSMC fan adapter

Declare the public IOKit connection functions and the private user-client struct. Use the generated IOKit declarations if the selected binding exposes these exact signatures; otherwise keep the small binding local:

```rust
type MachPort = u32;
type IoObject = u32;
type IoService = IoObject;
type IoConnect = IoObject;
type KernReturn = i32;

unsafe extern "C" {
    fn IOServiceMatching(name: *const core::ffi::c_char) -> CFMutableDictionaryRef;
    fn IOServiceGetMatchingService(
        main_port: MachPort,
        matching: CFDictionaryRef,
    ) -> IoService;
    fn IOServiceOpen(
        service: IoService,
        owning_task: MachPort,
        type_: u32,
        connect: *mut IoConnect,
    ) -> KernReturn;
    fn IOServiceClose(connect: IoConnect) -> KernReturn;
    fn IOObjectRelease(object: IoObject) -> KernReturn;
    fn IOConnectCallStructMethod(
        connection: IoConnect,
        selector: u32,
        input: *const core::ffi::c_void,
        input_size: usize,
        output: *mut core::ffi::c_void,
        output_size: *mut usize,
    ) -> KernReturn;
}
```

The C layout must be asserted at compile time:

```rust
#[repr(C)]
#[derive(Clone, Copy)]
struct SmcVersion {
    major: u8,
    minor: u8,
    build: u8,
    reserved: u8,
    release: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct SmcPLimitData {
    version: u16,
    length: u16,
    cpu_plimit: u32,
    gpu_plimit: u32,
    mem_plimit: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct SmcKeyInfoData {
    data_size: u32,
    data_type: u32,
    data_attributes: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct SmcParamStruct {
    key: u32,
    version: SmcVersion,
    p_limit_data: SmcPLimitData,
    key_info: SmcKeyInfoData,
    result: u8,
    status: u8,
    command: u8,
    data32: u32,
    bytes: [u8; 32],
}

const _: () = {
    use core::mem::{align_of, offset_of, size_of};

    assert!(size_of::<SmcVersion>() == 6);
    assert!(size_of::<SmcPLimitData>() == 16);
    assert!(size_of::<SmcKeyInfoData>() == 12);
    assert!(size_of::<SmcParamStruct>() == 80);
    assert!(align_of::<SmcParamStruct>() == 4);
    assert!(offset_of!(SmcParamStruct, key) == 0);
    assert!(offset_of!(SmcParamStruct, version) == 4);
    assert!(offset_of!(SmcParamStruct, p_limit_data) == 12);
    assert!(offset_of!(SmcParamStruct, key_info) == 28);
    assert!(offset_of!(SmcParamStruct, key_info.data_size) == 28);
    assert!(offset_of!(SmcParamStruct, result) == 40);
    assert!(offset_of!(SmcParamStruct, status) == 41);
    assert!(offset_of!(SmcParamStruct, command) == 42);
    assert!(offset_of!(SmcParamStruct, data32) == 44);
    assert!(offset_of!(SmcParamStruct, bytes) == 48);
};
```

If any assertion fails, do not compile the private backend. Do not use `repr(packed)`; unaligned field access would introduce undefined behavior. Do not derive or call `Default` for an FFI buffer: field-by-field initialization can leave ABI padding indeterminate. Keep `MaybeUninit::<SmcParamStruct>::zeroed()` in place and set/read fields through its pointer, calling `assume_init_ref()` only after a successful 80-byte output, or operate on a zeroed `[u8; 80]` with checked offset helpers. This keeps every byte sent to the kernel initialized.

Open sequence:

1. `IOServiceMatching(c"AppleSMC".as_ptr())`; null means Unsupported. The `c"..."` literal supplies the required terminal NUL.
2. `IOServiceGetMatchingService(kIOMainPortDefault, matching)`. This call consumes the matching dictionary even if no service is found; do not `CFRelease` it afterward.
3. `IOServiceOpen(service, unsafe { libc::mach_task_self_ }, 0, &mut connection)`. `mach_task_self_` is the exported task-port variable in the current `libc` crate, not a Rust function call.
4. Release the service object immediately after open; retain `IoConnect` until shutdown.
5. Every call uses `IOConnectCallStructMethod(connection, 2, ...)` with 80-byte input/output storage and initialized output size 80. After the call, require `IOReturn == 0`, returned output size exactly 80, and the firmware `result == 0` before reading any other output field.

Read sequence for a four-character key:

1. Encode key with `u32::from_be_bytes(*b"FNum")`.
2. Zero input/output; set key and command `9` (read key info).
3. Call selector 2; require the transport, output-size, and firmware-result checks above.
4. Validate `data_size` is 1–32 and decode the FourCC with
   `let type_code: [u8; 4] = output.key_info.data_type.to_be_bytes();`.
   Match that exact byte array together with declared size; do not convert the
   native integer through host-endian bytes or the type appears reversed.
5. Zero a second input, set key, **copy size into `key_info.data_size` at offset 28**, and set command `5` (read bytes).
6. Call again and validate all three layers. Putting the size in `data32` instead of `key_info.data_size` returned firmware result `0x89` locally; make the offset a fixture assertion, not a guessed field.
7. Decode only recognized type/size pairs.

Decoder table:

| SMC type | Decode |
|---|---|
| `ui8 ` | first byte unsigned |
| `ui16` | two-byte big-endian unsigned |
| `ui32` | four-byte big-endian unsigned |
| `flt ` | four-byte little-endian IEEE-754 on this M2; require finite/plausible |
| `fpe2` | big-endian unsigned 16-bit divided by 4 (older fan keys) |
| `sp78` | big-endian signed 16-bit divided by 256 (common temperatures) |

Map firmware result bytes without collapsing them into transport failures: `0x00` success, `0x01` generic error, `0x80` collision, `0x81` spurious data, `0x82` bad command, `0x83` bad parameter, `0x84` key not found, `0x85` key not readable, `0x86` key not writable, `0x87` size mismatch, `0x88` framing error, and `0x89` bad argument. Preserve an unknown byte numerically in an internal sanitized error. Authorization failure is a separate IOKit transport result such as `kIOReturnNotPrivileged` (`0xe00002c2`), not firmware result `0x89`.

Read `FNum`, require `ui8 ` size 1, and reject values above the defensive cap of 16. If zero, emit NotPresent. For each `i`, use the single uppercase hexadecimal index character (`0`–`F`) and probe only this read allowlist: `F{i}Ac` actual, `F{i}Tg` target, `F{i}Mn` minimum, `F{i}Mx` maximum, then both `F{i}Md` and `F{i}md` for mode compatibility. Preserve missing individual keys. On this M2 uppercase `F0Md` worked while `F0md` and `Ftst` did not. Do not enumerate the whole keyspace.

Validate `0 <= RPM <= 20,000`, `min <= max`, and nonzero actual/target values in a tolerance-expanded range; invalid data is ParseFailed, not zero. Special-case an explicit finite actual value of exactly zero as `Availability::Available + Some(0)`/stopped even when the advertised running minimum is nonzero. Do not apply that exception to a missing key, parse failure, target, or unknown mode. Serialize all SMC calls on the Mac worker, cache successful key metadata for the connection lifetime, and close/re-probe after sleep/wake or repeated transport failure. Close with `IOServiceClose`. Expose **no write command or public raw connection** in the crate: the only permitted firmware commands are key-info `9` and read-bytes `5`, through selector 2. Add a source-level test that the FFI module contains no write command/method/selector path.

The synthetic M2 fan fixture should be exactly reproducible without containing a machine dump:

| Key/type/bytes | Expected normalized value |
|---|---:|
| `FNum`, `ui8 `, `01` | 1 fan |
| `F0Ac`, `flt `, `cc 90 3a 45` | approximately 2985.0498 RPM |
| `F0Mn`, `flt `, `00 e0 95 44` | 1199 RPM |
| `F0Mx`, `flt `, `00 f8 e0 45` | 7199 RPM |
| `F0Tg`, `flt `, `00 60 3b 45` | 2998 RPM |
| `F0Md`, `ui8 `, `00` | automatic mode |

### IOReport dynamic adapter

Load `/usr/lib/libIOReport.dylib` using `dlopen(RTLD_LAZY | RTLD_LOCAL)` and resolve each required symbol into an `IoReportSymbols` table. If any symbol required for the selected metric is absent, mark only that candidate Unsupported and fall back.

Use these local ABI declarations; IOReport has no supported SDK header, so every symbol remains version-probed:

```rust
#[repr(C)]
struct __IOReportSubscription { _private: [u8; 0] }
type IOReportSubscriptionRef = *const __IOReportSubscription;

type CopyAllChannelsFn =
    unsafe extern "C" fn(u64, u64) -> CFDictionaryRef; // debug probe only
type CopyChannelsInGroupFn = unsafe extern "C" fn(
    CFStringRef, CFStringRef, u64, u64, u64,
) -> CFMutableDictionaryRef;
type MergeChannelsFn = unsafe extern "C" fn(
    CFMutableDictionaryRef, CFDictionaryRef, CFTypeRef,
);
type CreateSubscriptionFn = unsafe extern "C" fn(
    CFAllocatorRef,
    CFMutableDictionaryRef,
    *mut CFMutableDictionaryRef,
    u64,
    CFTypeRef,
) -> IOReportSubscriptionRef;
type CreateSamplesFn = unsafe extern "C" fn(
    IOReportSubscriptionRef, CFDictionaryRef, CFTypeRef,
) -> CFDictionaryRef;
type CreateSamplesDeltaFn = unsafe extern "C" fn(
    CFDictionaryRef, CFDictionaryRef, CFTypeRef,
) -> CFDictionaryRef;
type ChannelStringFn = unsafe extern "C" fn(CFDictionaryRef) -> CFStringRef;
type SimpleIntegerFn = unsafe extern "C" fn(CFDictionaryRef, i32) -> i64;
type StateCountFn = unsafe extern "C" fn(CFDictionaryRef) -> i32;
type StateNameFn = unsafe extern "C" fn(CFDictionaryRef, i32) -> CFStringRef;
type StateResidencyFn = unsafe extern "C" fn(CFDictionaryRef, i32) -> i64;
```

Resolve `IOReportCopyChannelsInGroup`, `IOReportMergeChannels`, `IOReportCreateSubscription`, `IOReportCreateSamples`, `IOReportCreateSamplesDelta`, `IOReportChannelGetGroup`, `IOReportChannelGetSubGroup`, `IOReportChannelGetChannelName`, `IOReportChannelGetUnitLabel`, `IOReportSimpleGetIntegerValue`, `IOReportStateGetCount`, `IOReportStateGetNameForIndex`, and `IOReportStateGetResidency`. Resolve `IOReportCopyAllChannels` only for the explicit capability/debug command. Initialize the subscription's returned-dictionary out pointer to null and reject a null subscription or null returned dictionary.

The safe wrapper owns:

- a copied channel dictionary;
- an IOReport subscription object;
- the subscribed-channel dictionary returned by creation (non-null and a distinct object from the requested dictionary on this host);
- previous sample and monotonic timestamp;
- a model-specific DVFS mapping;
- an allowlist of `(group, subgroup, channel)` selectors.

Never call `IOReportCopyAllChannels` during recurring collection. Use it only in an explicit local debug/capability command because the observed object was 4.68 MB and contained 7,923 channels. Normal initialization calls `IOReportCopyChannelsInGroup` for a narrow group, merges only requested channels, and creates one subscription.

Sampling algorithm:

1. `CreateSamples(subscription, subscribed_channels, null)` to establish baseline. Always use the dictionary returned by subscription creation, not the originally requested dictionary.
2. At the next scheduled monotonic instant, create a second sample.
3. `CreateSamplesDelta(first, second, null)`.
4. For simple counters call `IOReportSimpleGetIntegerValue(channel, 0)`.
5. For state channels iterate `0..IOReportStateGetCount(channel)`, read state name and residency.
6. Calculate energy/rate using the actual elapsed interval and source unit; reject zero/negative/implausibly long windows.
7. Replace baseline, release old CF objects, and publish.

Do not sleep inside the collector. The scheduler supplies the next sample; the first refresh returns `Availability::WarmingUp { expected_window_ms }`, and later refreshes delta the stored baseline. Discard that baseline after sleep/wake, a counter decrease, or a window outside the configured tolerance.

Convert only an exact recognized unit:

```rust
fn energy_joules(raw: i64, unit: &str) -> Option<f64> {
    if raw < 0 {
        return None;
    }
    match unit.trim() {
        "mJ" => Some(raw as f64 / 1_000.0),
        "uJ" => Some(raw as f64 / 1_000_000.0),
        "nJ" => Some(raw as f64 / 1_000_000_000.0),
        _ => None,
    }
}
```

Then `watts = joules / actual_elapsed_seconds`, with checked finite arithmetic. For this host, one loaded CPU Energy window was `14,905 mJ / 1.008 s = 14.8 W`. A shorter validation fixture observed `2,362 mJ / 0.20 s = 11.81 W` for CPU and `239,686,440 nJ / 0.20 s = 1.1984322 W` for GPU; these prove unit handling, not expected workload constants. Do not hard-code mJ solely from the channel name—validate the unit metadata.

Residency reported `24Mticks`; validate state-residency sums near the expected ticks/window before converting to a share, but do not reinterpret those ticks as nanoseconds. Publish three distinct quantities when mappings are complete:

```text
active_ratio = active_ticks / total_ticks
active_average_hz = sum(mapped_hz * mapped_active_ticks) / mapped_active_ticks
capacity_normalized_load = sum(mapped_hz * mapped_active_ticks)
                           / (total_ticks * maximum_hz)
```

The second is an interval-weighted active frequency, not an instantaneous clock. The third is effective/capacity-normalized load, not OS CPU utilization. Observed state labels were E CPU `IDLE,V0P6,...,V6P0`, P CPU `IDLE,V0P16,...,V16P0`, and GPU `OFF,P1,...,P15`. Parse the CPU `V` index and GPU `P` number instead of assuming that the state array offset equals a frequency-table offset. The local GPU table had entries only at indexes 0–8 (`off`, P1–P8); P9–P15 had zero residency. Map a GPU state only when `n < table.len()` and the table frequency is nonzero. If an unmappable P9+ ever has nonzero residency, preserve utilization/residency but return frequency unavailable—never index out of bounds or extrapolate.

The subscription, every object returned by a `Copy`/`Create`, the requested/returned dictionaries, samples, and delta are CF objects with independent lifetimes. The returned subscribed dictionary was distinct from the requested dictionary. Dictionary/array values and strings returned by `IOReportChannelGet*`/`IOReportStateGetNameForIndex` are borrowed and must not be released. Release every owned object exactly once after its last dependent sample, and keep the `libloading::Library` alive until after all IOReport objects are released; local teardown with `CFRelease` completed cleanly.

Create separate narrow requests for exactly `Energy Model`/null subgroup, `CPU Stats`/`CPU Core Performance States`, and `GPU Stats`/`GPU Performance States`; merge them only at initialization. Do not subscribe to `CPU Complex Performance States`: it yielded misleading all-100-style values in the local exploration and was not validated as a utilization signal.

`CopyChannelsInGroup` is still broader than the desired subscription. Its
returned dictionary contains a CFArray under the exact key
`IOReportChannels`; the local `Energy Model` request alone contained 136
channels. Filter **before** subscription with this algorithm:

1. Obtain the group's dictionary and type-check `IOReportChannels` as CFArray.
2. Create a new `CFMutableArray` with `kCFTypeArrayCallBacks`, so appended
   channel dictionaries are retained.
3. Iterate borrowed channel dictionaries from the source array. Type-check each
   dictionary, call the group/subgroup/name/unit accessors, and append it only
   if all exact allowlist fields match.
4. Make a mutable copy of the group dictionary and replace its
   `IOReportChannels` value with the filtered array using
   `CFDictionarySetValue` and the exact CFString key.
5. Merge only these filtered group dictionaries into the final request.
6. Release the temporary mutable array and dictionaries after the merge or
   subscription has retained what it needs. Never release the borrowed channel
   dictionary separately.

The allowlist version observed on `Mac14,7`, macOS build `25D2128`, is:

| Group | Subgroup | Channel | Unit | Shape |
|---|---|---|---|---|
| `Energy Model` | null | `CPU Energy` | `mJ` | simple delta |
| `Energy Model` | null | `GPU SRAM` | `mJ` | simple delta |
| `Energy Model` | null | `ANE` | `mJ` | simple delta |
| `Energy Model` | null | `DRAM` | `mJ` | simple delta |
| `Energy Model` | null | `GPU Energy` | `nJ` | simple delta |
| `CPU Stats` | `CPU Core Performance States` | `ECPU0` through `ECPU3` | `24Mticks` | 8 states: `IDLE,V0P6..V6P0` |
| `CPU Stats` | `CPU Core Performance States` | `PCPU0` through `PCPU3` | `24Mticks` | 18 states: `IDLE,V0P16..V16P0` |
| `GPU Stats` | `GPU Performance States` | `GPUPH` | `24Mticks` | 16 states: `OFF,P1..P15` |

`System Energy` was **absent** from this run and is not in the version-1
allowlist. Do not use a substring match, silently substitute another rail, or
broaden the request when a named channel is missing. After filtering, require
the exact expected channel count, units, and state labels. A mismatch makes
only that metric Unsupported/ProtocolMismatch, records the sanitized observed
shape for a capability test, and leaves other IOReport metrics active.

Every allowlist entry must carry model family, tested OS-build range, expected
unit and state schema, parser version, and public/command fallback. A future
explicit capability-development command may inspect all groups after privacy
warning, but normal TUI, CLI snapshot, logs, and exports must never do so.

### DVFS table adapter

Enumerate `AppleARMIODevice` services and select the entry whose registry name is exactly `pmgr`; do not hard-code an `IODeviceTree:/...` path. Copy only an allowlist of `voltage-states*`/`voltage-states*-sram` properties and require CFData. Reject null, a byte length of zero or over 4096, or a length not divisible by 8. Decode each record as little-endian `(frequency_raw: u32, second_raw_word: u32)` without sorting, deduplicating, or silently deleting entries.

Normalize frequency by magnitude only after retaining the raw word: values at least `100_000_000` are Hz here; values at least `100_000` may be kHz; values at least `100` may be MHz. Multiply with checked arithmetic, then require active states to fall within 100–10,000 MHz and be monotonically nondecreasing. Permit zero only as a leading off state. Reject the property rather than guess if more than one scale fits the validation constraints. Do not label the second word as voltage without a model/property rule.

On this exact M2 mapping:

```text
E CPU -> voltage-states1-sram
P CPU -> voltage-states5-sram
GPU   -> voltage-states9
```

Do not use those property numbers without a model mapping. Discover candidates, compare state counts/order with IOReport labels, and mark the mapping experimental. On newer Apple Silicon, probe `acc-clusters` as a bounded byte mapping: byte 0 has been used as the property index and byte 1 as the E/M/P performance tier in current community implementations, but accept it only when the referenced table exists and the IOReport state count corroborates it. This host had only E/P CPU tiers. Store Hz internally; convert to MHz only for display.

Parse CPU state names as `V<index>P<suffix>` and use only `<index>` as the DVFS-table index; parse GPU states as `P<n>`. Never use the array position as an implicit table index. A state-residency weighted active average is:

```text
sum(state_frequency_hz * state_residency_ticks) / sum(residency_ticks)
```

Exclude CPU `IDLE` and GPU `OFF` from average-active frequency but include them in utilization/residency proportions. The local E table had indices 0–6, P 0–16, and GPU off plus P1–P8; IOReport advertised GPU P9–P15 but those residencies were zero. Map only an index that exists and has a nonzero active frequency. If a future unmapped state has nonzero residency, preserve that residency/utilization and set frequency to unavailable rather than indexing out of bounds or extrapolating.

### Public Metal and experimental AGX adapters

Use `objc2-metal` on the Mac thread. At static/on-change refresh:

1. call `MTLCopyAllDevices`; use `MTLCreateSystemDefaultDevice` only as a last fallback because selecting the default device can wake/switch to a high-power GPU on dual-GPU Intel Macs;
2. produce one `GpuDevice` per returned device;
3. copy `name`, `registryID`, `location/locationNumber` where public and safe internally, `isLowPower`, `isHeadless`, `isRemovable`, `hasUnifiedMemory`, `recommendedMaxWorkingSetSize`, `maxBufferLength`, architecture name, and selected `supportsFamily` flags;
4. replace registry IDs with per-export pseudonyms;
5. do not sum recommended working sets or call them VRAM;
6. sample `currentAllocatedSize` only as “allocated by this process/device context,” never global use.

Core count is not a general public `MTLDevice` field. Obtain it from a separately sourced allowlisted profiler/registry observation and preserve that source.

Metal counter sample buffers can measure workloads SD-300 submits; they are not a passive system-wide monitor. Do not create dummy GPU work to infer global health.

For the optional AGX registry source:

1. match `IOAccelerator`/AGX services and require Apple/AGX identity;
2. copy `PerformanceStatistics` only;
3. allowlist numeric keys such as Device/Renderer/Tiler Utilization, allocated/in-use system memory, parameter-buffer memory, and recovery count/time;
4. reject out-of-range utilization and negative/decreasing non-reset counters;
5. refresh at 2–5 seconds, never enumerate the entire registry;
6. label source `agx_ioreg_private` and experimental;
7. prefer IOReport when both pass validity checks, and compare them in debug builds without silently blending definitions.

### IOPowerSources basic battery adapter

Link IOKit and CoreFoundation and declare/use:

```text
IOPSCopyPowerSourcesInfo() -> owned CFTypeRef
IOPSCopyPowerSourcesList(info) -> owned CFArrayRef
IOPSGetPowerSourceDescription(info, source) -> borrowed CFDictionaryRef
IOPSGetTimeRemainingEstimate() -> CFTimeInterval
IOPSCopyExternalPowerAdapterDetails() -> owned CFDictionaryRef
IOPSNotificationCreateRunLoopSource(callback, context) -> owned CFRunLoopSourceRef
```

Collection algorithm:

1. copy the info blob and source list;
2. for each source, obtain the borrowed description dictionary;
3. read only documented keys with CF type checks: power-source state, current/max capacity, charging/charged/present, time-to-empty/time-to-full, voltage/current/temperature/health fields only where the published key defines them, and safe type/transport;
4. normalize percent only when max > 0 and clamp after recording invalid raw-state diagnostics;
5. map negative or source-defined sentinel time to unavailable;
6. determine AC/battery from the source-state key, not `is_charging` alone (a full battery on AC may not be charging);
7. release list/info after copying Rust values;
8. copy external-adapter details and retain only rated watts/source state—exclude adapter IDs/serials;
9. register the notification source so source/charge transitions trigger refresh.

The C description dictionary is borrowed from the info blob. The generated objc2 wrapper may retain it for return safety, but keep the blob alive for snapshot consistency and follow the wrapper’s exact ownership signature. Capacity units vary; calculate percent from current/max but do not automatically label raw capacity mAh.

### AppleSmartBattery enhanced adapter

Match `AppleSmartBattery`, call `IORegistryEntryCreateCFProperties`, then immediately construct an allowlisted struct. Never serialize the source dictionary.

Candidate allowlist with validation:

| Field family | Example properties | Validation/normalization |
|---|---|---|
| Capacity | current/max/design/raw capacities | nonnegative; max/design > 0; keep raw + derived ratio |
| Health | cycle count, condition, failure flags | bounds and Apple string/bit semantics; no invented failure |
| Electrical | voltage, amperage, cell voltages | validate units per model/property; current sign convention captured |
| Temperature | `Temperature`, `VirtualTemperature` | retain raw encoding/source; decode only with cross-field plausibility |
| Adapter | connected, rated/negotiated voltage/current/power | rated, negotiated ceiling, and live draw are separate fields |
| Lifetime/telemetry | extrema/accumulators | hidden behind experimental detail until units documented |

This host’s raw temperature near 3109 aligns with roughly 37.8°C when interpreted as deci-Kelvin and with the separate gas-gauge reading. That is corroboration, not permission to apply the conversion to every key/model. Put decoding rules in a `(property, model family, OS range)` table and retain UnknownEncoding otherwise.

Treat `65535` time values as unavailable. Exclude serial/device/color/owner-related properties before any logging. Release the service object and copied property dictionary.

### Power assertions and optional `powermetrics`

Call `IOPMCopyAssertionsStatus` for aggregate assertion classes; copy only assertion type/count or user-safe explanations. A detailed process assertion list can expose app names and should be technician opt-in.

Implement `powermetrics` as an optional request adapter, not the normal collector:

1. verify `/usr/bin/powermetrics` and current `-h` sampler names;
2. require explicit helper authorization;
3. pass a fixed sampler allowlist, sample count/rate, and plist format—no user-provided arbitrary arguments;
4. parse NUL-separated plist records incrementally with an output ceiling;
5. terminate after the requested count/timeout;
6. strip process-identifying records unless explicitly requested;
7. distinguish authorization denied, unsupported sampler, empty-success, malformed plist, and timeout;
8. include Apple’s estimated-power caveat in the observation metadata.

### Disk Arbitration and APFS adapters

Use a Disk Arbitration session for device lifecycle and structured `diskutil` only for APFS details that public bindings do not conveniently expose.

Long-lived public path:

1. `DASessionCreate`.
2. Register disk-appeared, disk-disappeared, and description-changed callbacks.
3. Schedule on the Mac worker’s CFRunLoop or dispatch queue.
4. In each callback, copy `DADiskCopyDescription`; extract BSD name internally, media name/kind, protocol/path, size, internal/removable/ejectable/writable/whole/media-leaf flags, filesystem kind, volume name, and mount URL with sensitivity tagging. The public Disk Arbitration key set does **not** define an APFS volume-role key.
5. Use stable internal IDs for graph joins; pseudonymize or remove them at export.
6. Never mount/unmount/eject/repair from the monitor.

APFS fallback jobs execute absolute commands:

```text
/usr/sbin/diskutil list -plist
/usr/sbin/diskutil apfs list -plist
/usr/sbin/diskutil info -plist <validated BSD identifier>
```

Parse with the Rust `plist` crate into explicit serde structs. Reject unknown top-level type/oversized arrays; tolerate additive keys with `#[serde(default)]`; store schema/parser version. Build graph edges only from identifiers found in the structured payload. Never pass volume labels or arbitrary user strings as command arguments.

Read APFS roles only from the structured `Roles` array under
`diskutil apfs list -plist` volumes. Do not invent a `DADiskCopyDescription`
role constant or infer role from mount path/name. A missing `Roles` array means
role unavailable, not `Data` or `System` by default.

### Block-driver I/O counter adapter

Match `IOBlockStorageDriver` services and copy the `Statistics` dictionary. Allowlist cumulative counter keys for bytes/operations read/written, errors/retries, and time fields. Join a driver to a physical disk through registry parent/path or BSD-client relationship, not array order.

For each device/counter retain `(value, monotonic timestamp, registry generation)`. Rate logic:

```text
if same generation and new >= old and dt within budget:
    rate = (new - old) / dt
else:
    discard baseline; observation = Stale for one interval
```

Use checked arithmetic. Counter absence is Unsupported for that metric, not zero. Do not interpret time fields until their unit is confirmed from the SDK/source. Device disappearance deletes the baseline.

### SMART/NVMe selection logic

1. Obtain Apple `SMARTStatus`, TRIM, firmware/revision, protocol, and media class from structured Disk Arbitration/profiler/IOKit sources.
2. Report SMART availability and status separately.
3. For each validated whole-disk BSD identifier, parse
   `diskutil info -plist` and feature-probe the exact
   `SMARTDeviceSpecificKeysMayVaryNotGuaranteed` dictionary. Type-check every
   scalar; never deserialize it as an unbounded generic map for export.
4. Preserve each `_0` and `_1` `u64` source value, then derive a checked `u128`
   only through the versioned low/high-limb hypothesis. Mark that derivation
   experimental until another OS/model or authoritative Apple contract confirms
   it. Convert data units with checked `* 512_000`; convert composite Kelvin only
   when plausible; keep percent-used and spare semantics distinct.
5. Treat missing detailed keys as Unsupported for that field while retaining
   coarse SMART status. Treat wrong types, implausible Kelvin, impossible
   counters, or inconsistent limbs as ProtocolMismatch/Suspect—not zero.
6. Probe optional `/usr/local/bin/smartctl`, `/opt/homebrew/bin/smartctl`, and an
   administrator-configured absolute path only; never mutable `PATH` and never
   auto-install.
7. Run `smartctl` JSON with bounded privilege/timeout only on an explicitly
   selected physical device. Redact serial/WWN/model identifiers before
   logs/exports, and map attributes only when present and unit-defined.
8. If both Apple detail and passthrough are absent, return Unsupported with the
   coarse Apple SMART observation still available.

### CoreWLAN adapter

Use `objc2-core-wlan 0.3.x` with features `CWWiFiClient`, `CWInterface`, `CWChannel`, and `CoreWLANTypes`. Create `CWWiFiClient::sharedWiFiClient()` once on the Mac thread; call `interface()`/`interfaces()` and retain the objects there.

At 2–10-second cadence or link notification, collect:

- interface name and `powerOn`;
- RSSI, noise, transmit rate;
- `wlanChannel()` number/band/width;
- PHY/interface mode and security enums;
- country code availability;
- SSID/BSSID **availability state**, not necessarily their values;
- optionally cached scan-result count, not names.

Bounds-check sentinel values (CoreWLAN methods can return special negative/zero values when unavailable). Do not initiate active scans by default. Register only useful link/power/mode events via `startMonitoringEventWithType:error:` and unregister on shutdown. The delegate must remain alive while registered and stay on the same worker/thread as required.

Fields policy:

```text
RSSI/noise/rate/channel/security -> safe ephemeral technician data
SSID/BSSID/country/hardware address/scan entries -> sensitive, TCC-aware
```

When SSID/BSSID is nil, record PermissionDenied only if authorization evidence says so; otherwise Unsupported/Unavailable with a safe reason. Never infer “not connected” from identity being withheld.

### Interface/path/counter adapter

Retain the current `sysinfo::Networks` delta path initially, but add a native interface descriptor map:

1. call `getifaddrs`; copy name, flags, address/netmask/destination for AF_INET/AF_INET6/AF_LINK;
2. group rows by name and free the list with `freeifaddrs`;
3. classify loopback, point-to-point, broadcast, multicast, up/running;
4. obtain MTU/type/link rate/counters through `sysctl`/IOKit/SystemConfiguration as available;
5. map interface to network service through a long-lived `SCDynamicStore`, querying exact `State:/Network/Global/IPv4`, IPv6, DNS, Proxies, and per-interface Link/IP keys rather than interpreting `en0` text;
6. register exact dynamic-store keys/patterns on one serial dispatch queue; callbacks receive borrowed changed-key arrays and should only mark dirty/re-query; call `set_dispatch_queue(None)` before releasing the store/queue/context;
7. consume `NWPathMonitor` or `SCNetworkReachability` on a dispatch queue for satisfied/constrained/expensive/interface-type state;
8. keep default route, DNS, proxies, and active reachability as separate observations.

Never export AF_LINK/MAC or addresses by default. Hashing a stable MAC with no salt is still identifying; omit or use a random per-export token.

### Active probe adapter

Represent probes as explicit `DiagnosticRequest` variants, not automatic refresh:

```rust
enum ActiveProbe {
    Gateway,
    Dns { hostname: ApprovedHost },
    Https { url: ApprovedUrl },
    Icmp { host: ApprovedHost },
}
```

Prefer Rust DNS/TCP/TLS libraries over platform `ping`. If `ping` remains, use a per-OS argument builder and parse RTT lines from controlled locale; a process timeout is a safety limit, not the network RTT. Record that active traffic occurred.

### Process/libproc adapter

Keep `sysinfo` for the initial process index and remove the pre-sort
truncation, but do not use it for total threads: its macOS process task list is
unavailable. Add a checked `libproc` binding behind `MacProcessApi`:

```rust
use std::ffi::c_void;

const PROC_PIDTASKINFO: i32 = 4;

unsafe extern "C" {
    fn proc_listallpids(buffer: *mut c_void, buffersize: i32) -> i32;
    fn proc_pidinfo(
        pid: i32,
        flavor: i32,
        arg: u64,
        buffer: *mut c_void,
        buffersize: i32,
    ) -> i32;
}

#[repr(C)]
#[derive(Clone, Copy)]
struct ProcTaskInfo {
    pti_virtual_size: u64,
    pti_resident_size: u64,
    pti_total_user: u64,
    pti_total_system: u64,
    pti_threads_user: u64,
    pti_threads_system: u64,
    pti_policy: i32,
    pti_faults: i32,
    pti_pageins: i32,
    pti_cow_faults: i32,
    pti_messages_sent: i32,
    pti_messages_received: i32,
    pti_syscalls_mach: i32,
    pti_syscalls_unix: i32,
    pti_csw: i32,
    pti_threadnum: i32,
    pti_numrunning: i32,
    pti_priority: i32,
}

const _: () = assert!(std::mem::size_of::<ProcTaskInfo>() == 96);
```

Call `proc_listallpids(null, 0)` to obtain a changing capacity hint, allocate
`hint + slack` elements with a hard cap, and pass
`element_capacity * size_of::<i32>()` as the byte `buffersize` to the second
call. Unlike several other libproc list APIs, `proc_listallpids` returns a **PID
element count**, not a byte count. Reject negative returns and truncate the
`Vec<i32>` directly to that count; discard PID `<= 0`; sort and deduplicate
because processes can churn between calls. If returned count equals element
capacity, retry once with a bounded larger allocation. A fresh local check
returned a 929-element hint and then 910 PIDs in a 1,057-element buffer,
illustrating why the two calls need not agree.

At a 3–10-second cadence, zero one `ProcTaskInfo` per PID and call
`proc_pidinfo(pid, PROC_PIDTASKINFO, 0, ..., 96)`. Accept the record only when
the return is exactly 96 bytes and `pti_threadnum >= 0`. Sum with checked `u64`
arithmetic and publish:

```text
known_threads = sum(pti_threadnum for successful PIDs)
coverage = successful_pid_records / attempted_live_pids
result kind = lower_bound, never exact total when coverage < 100%
```

A safe live sample on this host enumerated 837 PIDs: 423 task-info calls
succeeded, 413 were permission-denied, and one raced/exited or otherwise failed;
the known thread lower bound was 3,066. These values are workload snapshots,
not fixtures or thresholds. They prove why the UI must show “at least 3,066
threads (423/837 processes readable)” rather than either `0` or a false exact
total.

Every call can fail with `ESRCH` due process exit or `EPERM`/`EACCES` due
permissions. A race removes or marks only that row; a permission failure lowers
coverage and marks protected fields unavailable. Never fail the snapshot.
`proc_pid_rusage` may enrich versioned CPU/I/O data, and `proc_pidpath` may run
only after sensitive-data opt-in. Sample cumulative fields twice to produce
rates. Avoid enumerating all file descriptors/sockets each second; enrich only
visible/selected/top rows asynchronously.

### CoreGraphics display adapter and optional AppKit broker

The supported CLI/TUI path is CoreGraphics-only. Apple's [`NSScreen`](https://developer.apple.com/documentation/appkit/nsscreen) overview requires creating `NSApplication` before using screens so AppKit can connect to the window server, and AppKit lifecycle belongs on the process main thread. Do not create `NSApplication` on the dedicated collector worker: doing so changes a terminal program's lifecycle and relies on unsupported behavior. Local Swift probes happened to return one `NSScreen` with `NSApp == nil`, even from a detached worker with an autorelease pool, but that is an observed tolerance—not a contract.

If the optional Native/GUI app later needs AppKit-only backing-scale, visible-frame, or EDR fields, initialize its application lifecycle on the main thread and expose a narrow main-thread `AppKitDisplayBroker` that returns owned DTOs to the collector. The TUI should mark those fields Unsupported rather than initialize AppKit.

At startup and after `CGDisplayRegisterReconfigurationCallback`:

1. call `CGGetOnlineDisplayList` with size query then allocated buffer;
2. for each `CGDirectDisplayID`, copy bounds, built-in/main/active/online/asleep/mirror/rotation, physical millimeters, current display mode, and all mode capabilities only on demand;
3. current mode stores logical width/height, pixel width/height, refresh, flags, and desktop usability separately;
4. retain vendor/model/serial only for in-memory matching; redact export;
5. callback posts a refresh command—do not perform heavy collection inside the callback.

Handle 0 Hz as “variable/unknown refresh,” not a literal stopped display. Do not call `CGDisplayPixelsWide` alone the resolution.

### CoreAudio adapter

Use the coherent `objc2-core-audio` generated bindings to `AudioObjectGetPropertyDataSize`, `AudioObjectGetPropertyData`, and C property listeners. Always perform a size query, allocate exact aligned storage, and retry once if the device-list size changes.

Static/on-change property selectors:

```text
kAudioHardwarePropertyDevices
kAudioHardwarePropertyDefaultInputDevice
kAudioHardwarePropertyDefaultOutputDevice
kAudioHardwarePropertyDefaultSystemOutputDevice
kAudioObjectPropertyName
kAudioDevicePropertyDeviceIsAlive
kAudioDevicePropertyDeviceIsRunningSomewhere
kAudioDevicePropertyTransportType
kAudioDevicePropertyNominalSampleRate
kAudioDevicePropertyAvailableNominalSampleRates
kAudioDevicePropertyBufferFrameSize
kAudioDevicePropertyStreamConfiguration
```

Query the correct input/output scope and master element for each property. For channel totals, request `kAudioDevicePropertyStreamConfiguration` separately under input and output scope, then sum `mNumberChannels` across the returned buffer list. Build `AudioDevice` with alive/running/default roles, transport, sample rates, buffer frames, latency/safety offset, and channel counts. Unsupported property/status applies only to that field. Audio object IDs are scalar handles and are not released; CFString-valued properties follow their header ownership and must be wrapped/released immediately. Register C hardware/device listeners with an exact address/callback/context tuple and unregister that tuple before the context drops. Callback threads should only `try_send` a dirty event.

Never open an `AudioUnit`, read microphone samples, or play audio during inventory. Explicit sound/microphone tests are separate commands with TCC explanation.

### AVFoundation camera adapter

Using `objc2-av-foundation`, enumerate video devices with `AVCaptureDeviceDiscoverySession` and record safe model/transport/position/connected/suspended fields and authorization status. Discard unique IDs from normal output. Do not create/start `AVCaptureSession` for inventory.

An explicit test follows: check/request authorization in a GUI-capable context, present visible preview/indicator, start one capture session, collect a frame only after consent, stop immediately, release. The CLI should not request camera permission during ordinary monitoring.

### IOKit device inventory and notifications

Build category-specific match dictionaries rather than dump `IOService`:

- USB host/device classes;
- Bluetooth controller only;
- storage/block/media;
- HID devices;
- accelerator/display services;
- network interfaces/controllers;
- Thunderbolt/PCI classes as validated.

For each class define an allowlist of property names and expected CF types. Traverse parents only to a bounded depth with a visited set. Store a per-run internal registry token; exclude path/serial/UUID by default.

Use `IONotificationPortCreate`, add first-match and terminated notifications, and **drain the initial iterator** to arm notifications. Dispatch callback work into the Mac worker; release every yielded `io_object_t`, iterator, and notification port. A device existing is inventory, not health—health requires driver/state/error properties with documented semantics.

Each matching dictionary is consumed by IOKit registration, so create a fresh one for matched and terminated notifications. Callbacks must not allocate heavily, block, run commands, render, or unwind across FFI. Wrap the Rust entry with `catch_unwind`, atomically note shutdown, and `try_send(DeviceClassDirty)`; coalesce/re-enumerate roughly 250 ms later on the worker.

Shutdown order is part of correctness:

1. set `shutting_down`;
2. unregister CoreAudio, CoreGraphics, and Disk Arbitration callbacks;
3. detach framework dispatch queues/run-loop sources;
4. release notification iterators;
5. destroy `IONotificationPort`;
6. drop retained sessions/queues;
7. drop the pinned callback context last.

### Security adapter

Keep this on-demand and command-based until a public native route clearly improves it. Use absolute paths, individual timeouts, and versioned parsers:

```text
/usr/bin/csrutil status
/usr/bin/csrutil authenticated-root status
/usr/bin/fdesetup isactive
/usr/bin/fdesetup status
/usr/sbin/spctl --status
/usr/bin/profiles status -type enrollment
/usr/bin/systemextensionsctl list
/usr/libexec/ApplicationFirewall/socketfilterfw --getglobalstate
/usr/libexec/ApplicationFirewall/socketfilterfw --getstealthmode
/usr/libexec/ApplicationFirewall/socketfilterfw --getloggingmode
/usr/bin/kmutil showloaded --list-only --no-kernel-components
```

Each command gets its own Availability. Prefer `fdesetup isactive` for the boolean and use `status` only for progress. Do not require Full Disk Access to read TCC databases; instead report that protected policy detail is unavailable. Enrollment, system-extension, and kernel-component names can reveal organizations/security products, so User Mode should show counts/status only and export should omit names.

### Command fallback implementation

Replace the current helper with an async/bounded interface:

```rust
struct CommandSpec {
    absolute_program: &'static Path,
    args: Vec<OsString>,
    timeout: Duration,
    stdout_limit: usize,
    stderr_limit: usize,
    environment: &'static [(&'static str, &'static str)],
    expected_format: OutputFormat,
}
```

Spawn with `tokio::process::Command`, `kill_on_drop(true)`, cleared/controlled environment, stdin null, and concurrent capped readers for stdout/stderr. On Unix, call its `process_group(0)` builder so a timeout can terminate descendants as well as the immediate PID; send group `SIGTERM`, allow a short bounded grace period, then group `SIGKILL`, and always `wait()` to reap. Keep `kill_on_drop` as a last-resort guard, not the normal cleanup path. Explicitly remove `DYLD_INSERT_LIBRARIES` and `DYLD_LIBRARY_PATH`, and set `LC_ALL=C`/`LANG=C` for text fallbacks. When either byte cap is exceeded, perform the same group cleanup and return `Availability::OutputTooLarge { limit_bytes, observed_at_least_bytes }`. On timeout, clean up/reap and return TimedOut. Validate format/schema even on exit 0. Only safe stderr snippets enter errors; a nonzero exit does not mean the queried feature is disabled.

Use command jobs only from background/on-demand workers. Cache static profiler results with an OS/model/tool-version key and invalidate on hardware notification.

### Sanitized real request/response payloads from this host

This section records **real schemas and non-identifying values** observed on the
borrowed Mac. It intentionally omits or replaces serial numbers, UUIDs, SSID,
BSSID/MAC, IP/router/DNS values, volume names, mount paths, local usernames,
Bluetooth device names, audio UIDs, and object keys derived from user device
names. Exact battery/disk lifetime counters can become a quasi-identifier in
combination, so product exports should coarsen them even when a research example
below preserves a useful point value.

The command runner's input and output should themselves be serializable test
fixtures. This is the exact logical request envelope used for the safe hardware
profiler example:

```json
{
  "program": "/usr/sbin/system_profiler",
  "args": ["-json", "SPHardwareDataType"],
  "environment_clear": true,
  "environment": {"LC_ALL": "C", "LANG": "C"},
  "stdin": "null",
  "timeout_ms": 5000,
  "stdout_limit_bytes": 262144,
  "stderr_limit_bytes": 32768,
  "expected_format": "json",
  "sensitivity_policy": "allowlisted_fields_only"
}
```

One clean-environment execution returned exit 0 in about 495 ms with 606 raw
stdout bytes. The raw output was parsed in memory and reduced to this safe
response payload before persistence:

```json
{
  "exit_code": 0,
  "timed_out": false,
  "stdout_truncated": false,
  "stderr_truncated": false,
  "schema": "system_profiler.SPHardwareDataType.v1",
  "payload": {
    "machine_name": "MacBook Pro",
    "machine_model": "Mac14,7",
    "chip_type": "Apple M2",
    "number_processors": "proc 8:4:4:0",
    "physical_memory": "8 GB",
    "activation_lock_status": "activation_lock_disabled"
  },
  "redacted_fields": [
    "model_number",
    "platform_UUID",
    "provisioning_UDID",
    "serial_number"
  ]
}
```

The program invocation is always
`/usr/sbin/system_profiler -json <one allowlisted data type>`, never a shell
string and never an unrestricted all-types capture. Initial budgets based on
this host are:

| Type | Timeout | Raw stdout cap | Observed clean-env time/size in one run | Persistent allowlist |
|---|---:|---:|---:|---|
| `SPHardwareDataType` | 5 s | 256 KiB | ~495 ms / 606 B | model class, chip, topology, memory, boot/loader versions, Activation Lock state |
| `SPDisplaysDataType` | 10 s | 512 KiB | ~506 ms / 1,243 B | GPU/display capability and mode tokens |
| `SPNVMeDataType` | 10 s | 512 KiB | ~199 ms / 1,346 B | controller/model class, revision, BSD join key, size, SMART/TRIM |
| `SPPowerDataType` | 5 s | 512 KiB | ~83 ms / 3,153 B | charge/health/cycles; no model/adapter serial |
| `SPAudioDataType` | 10 s | 1 MiB | not retained | safe capability fields only; names/UIDs sensitive |
| `SPBluetoothDataType` | 15 s | 1 MiB | ~189 ms / 4,634 B | controller-only allowlist; omit all paired/connected devices |
| `SPAirPortDataType` | 15 s | 1 MiB | ~6.7 s / 7,582 B | radio/link fields only; omit identities and scan graph |

Those durations are workload samples, not timeout promises. Profiler payloads
can be empty at exit 0, can gain fields, and can place personal device names in
**dictionary keys**. Parse from the top-level type array and construct a new
allowlisted DTO; never redact only values in the original JSON tree.

Safe real profiler response examples follow. Display:

```json
{
  "_name": "Apple M2",
  "sppci_cores": "10",
  "sppci_device_type": "spdisplays_gpu",
  "sppci_model": "Apple M2",
  "spdisplays_vendor": "sppci_vendor_Apple",
  "spdisplays_mtlgpufamilysupport": "spdisplays_metal4",
  "displays": [{
    "spdisplays_connection_type": "spdisplays_internal",
    "spdisplays_display_type": "spdisplays_built-in_retinaLCD",
    "spdisplays_pixelresolution": "spdisplays_2560x1600Retina",
    "spdisplays_main": "spdisplays_yes",
    "spdisplays_mirror": "spdisplays_off",
    "spdisplays_online": "spdisplays_yes"
  }]
}
```

NVMe profiler (the `device_serial` and nested volume identities were removed):

```json
{
  "controller": "Apple SSD Controller",
  "device": {
    "bsd_name": "disk0",
    "device_model": "APPLE SSD AP0512Z",
    "device_revision": "555",
    "size": "500.28 GB",
    "size_in_bytes": 500277792768,
    "smart_status": "Verified",
    "spnvme_trim_support": "Yes",
    "partition_map_type": "guid_partition_map_type",
    "detachable_drive": "no",
    "removable_media": "no"
  }
}
```

Power profiler (battery model and serial subtree omitted):

```json
{
  "charge": {
    "sppower_battery_state_of_charge": 100,
    "sppower_battery_fully_charged": "TRUE",
    "sppower_battery_is_charging": "FALSE",
    "sppower_battery_at_warn_level": "FALSE"
  },
  "health": {
    "sppower_battery_cycle_count": 114,
    "sppower_battery_health": "Good",
    "sppower_battery_health_maximum_capacity": "88%"
  }
}
```

Wi-Fi link (SSID/BSSID, interface MAC, country, and scan results omitted):

```json
{
  "status": "spairport_status_connected",
  "current": {
    "spairport_network_channel": "48 (5GHz, 80MHz)",
    "spairport_network_mcs": 8,
    "spairport_network_phymode": "802.11ac",
    "spairport_network_rate": 173,
    "spairport_network_type": "spairport_network_type_station",
    "spairport_security_mode": "spairport_security_mode_wpa2_personal",
    "spairport_signal_noise": "-57 dBm / -93 dBm"
  }
}
```

MCS/rate/RSSI varied substantially during this session; these fields require a
sample timestamp and must not be frozen as static capability. The payload also
contained a second mostly-null pseudo-interface, so retain an interface only
when it has a meaningful identity/state instead of taking array index zero.

Bluetooth controller (controller address and every device subtree omitted):

```json
{
  "controller_chipset": "BCM_4378B3",
  "controller_discoverable": "attrib_off",
  "controller_firmwareVersion": "23.1.152.291",
  "controller_productID": "0x4A0F",
  "controller_transport": "PCIe",
  "controller_vendorID": "0x004C (Apple)"
}
```

#### `diskutil` wire format and typed parser

The actual response from each `diskutil ... -plist` invocation is an XML plist,
not JSON. A successful response begins like this:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>...</dict></plist>
```

Parse those bytes directly with `plist`; JSON below is only a readable
representation. Real clean-environment requests observed here were:

| Request args | Exit/time/raw bytes | Exact safe top-level schema |
|---|---:|---|
| `list -plist` | 0 / ~112 ms / 12,566 B | `AllDisks`, `WholeDisks`, `VolumesFromDisks`, `AllDisksAndPartitions` |
| `info -plist disk0` | 0 / ~145 ms / 3,691 B | one device-description dictionary, including coarse SMART and optional private detail |
| `apfs list -plist` | 0 / ~202 ms / 10,064 B | `Containers` |

Use a 10-second timeout and 1 MiB cap initially, with a larger explicitly
tested cap only for machines with many external devices/volumes. Validate
`disk0` against `^disk[0-9]+$` after obtaining it from an enumerated whole-disk
record; never accept arbitrary user text.

Observed structural paths were:

```text
AllDisksAndPartitions[]
  Content, DeviceIdentifier, OSInternal, Size
  Partitions[]
    Content, DeviceIdentifier, DiskUUID, MountPoint, Size, VolumeName, VolumeUUID
  APFSPhysicalStores[].DeviceIdentifier
  APFSVolumes[]
    CapacityInUse, DeviceIdentifier, DiskUUID, MountPoint, MountedSnapshots,
    OSInternal, Size, VolumeName, VolumeUUID

Containers[]
  APFSContainerUUID, CapacityCeiling, CapacityFree, ContainerReference,
  DesignatedPhysicalStore
  PhysicalStores[]: DeviceIdentifier, DiskUUID, Size
  Volumes[]: APFSVolumeUUID, CapacityInUse, CapacityQuota, CapacityReserve,
             DeviceIdentifier, Encryption, FileVault, Name, Roles[]
```

Identifiers are necessary for the in-memory graph but sensitive in export.
Pseudonymize with a fresh random per-export salt **after** joins; do not delete
them before graph construction and do not use an unsalted stable hash.

The all-target parser can begin with explicit serde names such as:

```rust
#[derive(serde::Deserialize)]
struct DiskutilList {
    #[serde(rename = "AllDisks", default)]
    all_disks: Vec<String>,
    #[serde(rename = "WholeDisks", default)]
    whole_disks: Vec<String>,
    #[serde(rename = "VolumesFromDisks", default)]
    volumes_from_disks: Vec<String>,
    #[serde(rename = "AllDisksAndPartitions", default)]
    disks: Vec<DiskNode>,
}

#[derive(serde::Deserialize)]
struct DiskNode {
    #[serde(rename = "DeviceIdentifier")]
    device_identifier: String,
    #[serde(rename = "Content", default)]
    content: Option<String>,
    #[serde(rename = "Size", default)]
    size: Option<u64>,
    #[serde(rename = "OSInternal", default)]
    os_internal: Option<bool>,
    #[serde(rename = "Partitions", default)]
    partitions: Vec<DiskNode>,
    #[serde(rename = "APFSPhysicalStores", default)]
    apfs_physical_stores: Vec<DeviceReference>,
    #[serde(rename = "APFSVolumes", default)]
    apfs_volumes: Vec<ApfsVolumeReference>,
}

#[derive(serde::Deserialize)]
struct DiskutilInfo {
    #[serde(rename = "DeviceIdentifier")]
    device_identifier: String,
    #[serde(rename = "SMARTStatus", default)]
    smart_status: Option<String>,
    #[serde(rename = "SMARTDeviceSpecificKeysMayVaryNotGuaranteed", default)]
    smart_private: Option<AppleNvmeSmartRaw>,
}

#[derive(serde::Deserialize)]
struct AppleNvmeSmartRaw {
    #[serde(rename = "AVAILABLE_SPARE", default)]
    available_spare: Option<u64>,
    #[serde(rename = "AVAILABLE_SPARE_THRESHOLD", default)]
    available_spare_threshold: Option<u64>,
    #[serde(rename = "PERCENTAGE_USED", default)]
    percentage_used: Option<u64>,
    #[serde(rename = "TEMPERATURE", default)]
    temperature_kelvin: Option<u64>,
    #[serde(rename = "DATA_UNITS_READ_0", default)]
    data_units_read_low: Option<u64>,
    #[serde(rename = "DATA_UNITS_READ_1", default)]
    data_units_read_high: Option<u64>,
    // Repeat explicit fields for written, host commands, errors, log entries,
    // hours, cycles, unsafe shutdowns, and controller-busy time. Do not flatten.
}
```

The sanitized real SMART-detail response shape was:

```json
{
  "SMARTStatus": "Verified",
  "SMARTDeviceSpecificKeysMayVaryNotGuaranteed": {
    "AVAILABLE_SPARE": 100,
    "AVAILABLE_SPARE_THRESHOLD": 99,
    "PERCENTAGE_USED": 4,
    "TEMPERATURE": 332,
    "DATA_UNITS_READ_0": 707000000,
    "DATA_UNITS_READ_1": 0,
    "DATA_UNITS_WRITTEN_0": 252000000,
    "DATA_UNITS_WRITTEN_1": 0,
    "MEDIA_ERRORS_0": 0,
    "MEDIA_ERRORS_1": 0,
    "NUM_ERROR_INFO_LOG_ENTRIES_0": 0,
    "NUM_ERROR_INFO_LOG_ENTRIES_1": 0,
    "POWER_ON_HOURS_0": 3360,
    "POWER_ON_HOURS_1": 0,
    "POWER_CYCLES_0": 220,
    "POWER_CYCLES_1": 0,
    "UNSAFE_SHUTDOWNS_0": 14,
    "UNSAFE_SHUTDOWNS_1": 0,
    "CONTROLLER_BUSY_TIME_0": 0,
    "CONTROLLER_BUSY_TIME_1": 0
  }
}
```

Read/data counters and hours are deliberately rounded; parser fixtures should
use obvious synthetic integers and edge cases rather than this machine's
lifetime fingerprint.

#### Native request/response examples

The SMC section above already contains the exact 80-byte transaction layout,
field offsets, two-call field sequence, and real non-identifying fan payload
bytes; it does not claim to preserve full raw 80-byte machine dumps. The IOHID section contains the
matching dictionary (`PrimaryUsagePage=0xff00`, `PrimaryUsage=5`), event request
(`type=15`, field `983040`), service count, and validity examples. The IOReport
section contains the exact group/subgroup/channel selectors, sample/delta call
sequence, state labels, units, and real energy values. Preserve those as
transport fixtures rather than wrapping private APIs in command text.

For public I/O Registry block statistics, the logical request is
`IORegistryEntryCreateCFProperty(entry, CFSTR("Statistics"), ...)`. One primary
driver returned this safe, rounded representation during the session:

```json
{
  "Bytes (Read)": 2160000000000,
  "Bytes (Write)": 1110000000000,
  "Operations (Read)": 107000000,
  "Operations (Write)": 24300000,
  "Errors (Read)": 0,
  "Errors (Write)": 0,
  "Retries (Read)": 0,
  "Retries (Write)": 0,
  "Latency Time (Read)": 0,
  "Latency Time (Write)": 0,
  "Total Time (Read)": 375966358588987,
  "Total Time (Write)": 1994701982193
}
```

The totals changed markedly during the research/build workload. They are
cumulative counters, not speed, and the zero latency fields appear unpopulated
on this driver. Keep raw total-time semantics unknown until its unit is confirmed
for the source; derive rates only from timestamped deltas whose counters and
device generation are monotonic.

A flattened, versioned export record derived from `Observation<T>` should look
like this, making experimental provenance impossible to lose. It is not the
literal serde representation of the generic Rust struct above: the exporter
adds `metric`/`source_schema`, flattens enum unit variants, and renames
`window_ms` to `sample_window_ms`. Test both schemas independently.

```json
{
  "metric": "storage.nvme.percentage_used",
  "value": 4,
  "unit": "percent_life_consumed",
  "availability": "available",
  "validity": "valid",
  "freshness": "fresh",
  "source": "diskutil_info_private_smart_dictionary",
  "source_schema": "macos25.diskutil-smart.v1",
  "scope": "physical_device",
  "sensitivity": "hardware_lifecycle_quasi_identifier",
  "confidence": "exact_host_high_cross_model_low",
  "sample_window_ms": null
}
```

### Parser and fixture contracts for Windows development

Do not gate the entire semantic Mac collector behind `cfg(macos)`, because that would prevent Windows development from testing it. Split the boundary:

```text
macos_model.rs          # all targets: serde raw/normalized DTOs
macos_normalize.rs      # all targets: deltas, sentinels, privacy, health inputs
macos_command_parse.rs  # all targets: plist/JSON/text parsers
macos/live/             # cfg(macos): Apple handles and callbacks only
```

The live layer converts Apple objects immediately into owned, sensitivity-tagged `MacRaw*` DTOs. Normalizers and common `SystemSnapshot` conversion compile and test on Windows. No CF/Objective-C/Mach handle or pointer may appear in a DTO.

Do **not** commit raw host dumps. Create minimal synthetic fixtures that reproduce structure and edge cases:

```text
tests/fixtures/macos/
├── diskutil-list-basic.plist
├── diskutil-apfs-shared-container.plist
├── diskutil-info-sealed-snapshot.plist
├── profiler-power-sanitized.json
├── profiler-nvme-sanitized.json
├── profiler-bluetooth-controller-only.json
├── profiler-wifi-radio-only.json
├── ioreg-block-statistics-sanitized.plist
├── ioreg-agx-statistics-sanitized.plist
├── smc-fan-m2.bin             # constructed bytes, no machine output
├── ioreport-energy-delta-synthetic.plist
└── malformed/
```

Every fixture must be hand-rebuilt from allowed fields and use obviously synthetic labels/IDs. Add a test that scans fixtures for MAC, UUID, email, common home paths, IP, serial-like keys/values, and forbidden raw keys.

Required tests per adapter:

- happy path and every Availability state;
- missing/additive/wrong-type/empty-success fields;
- oversized payload and timeout;
- sentinel values (`65535`, zero temperature, nonfinite float);
- counter decrease/reset/wrap and device-generation change;
- timestamp/window staleness;
- P/E/GPU state-count mismatch;
- IOReport unit fixtures for `mJ`, `uJ`, and `nJ`; negative/unknown units, zero windows, zero active ticks, and nonzero unmapped states;
- IOReport ownership mocks proving each Copy/Create object is released once, borrowed values are not released, and samples use the returned subscribed dictionary even when it differs from the request;
- partial-null teardown and missing-library/missing-symbol capability degradation for every private backend;
- fanless `FNum=0`, multiple fans, missing target/min/max, the exact M2 `flt ` fixture above, `fpe2 [2e e0] = 3000`, `sp78 [19 80] = 25.5`, and `sp78 [ff 00] = -1`;
- SMC compile-time size/offset checks, command `9` then `5`, size written at offset 28, wrong-offset `0x89`, payload size 33 rejected before slicing, and transport-success/firmware-failure kept distinct;
- IOHID invalid negative channels and unmapped labels;
- APFS shared containers, snapshots, multiple physical stores, external devices;
- nil Wi-Fi identity with live radio/link;
- default audio/display changes during enumeration;
- redaction of identifiers in values **and object keys**;
- User Mode never derives Good/Quiet/Connected from unsupported data.

### CI and later Mac gates

Windows development can validate domain logic and fixtures, but not Apple ABI/runtime. Add gates:

1. Linux/Windows: all platform-neutral tests, fixture parsers, redaction, UI snapshots.
2. `cargo check --target aarch64-apple-darwin` and x86_64 where the toolchain permits; this catches Rust/cfg syntax but not runtime/link semantics.
3. Hosted macOS: native compile, framework link, public API smoke, sanitized no-private-required tests.
4. Physical Apple Silicon: IOHID/SMC/IOReport ignored tests with numeric plausibility and no identifier output.
5. Physical Intel/Rosetta/model fleet: matrix described above.
6. Signed/notarized app/CLI artifacts: repeat permission/private-access smoke from the **actual archive**, not only `cargo run`.

Private live tests must skip with an explicit capability reason, never pretend to pass. Save only sanitized assertion summaries: source available, count/range/unit checks, and error class.

### Definition of done for a metric

A metric is not implemented merely because a number appears. It is complete only when:

- source/access/capability detection exists;
- handle lifecycle and unsafe boundary are reviewed;
- scope, unit, sample window, timestamp, confidence, and sensitivity are populated;
- empty/denied/not-present/unsupported/stale/parse-failure states are distinct;
- counter reset and sleep/wake behavior are defined;
- cadence and timeout budget are enforced off the render loop;
- User and Technician wording is evidence-correct;
- export redaction is tested;
- synthetic fixture tests run cross-platform;
- the relevant real-Mac and signed-artifact gate passes.

---

## Vercel Labs Native SDK assessment

### Identity and current status

The user’s recollection was correct: [Vercel Labs Native](https://github.com/vercel-labs/native) is a Zig-heavy SDK for cross-platform desktop applications. As researched on 2026-07-17:

- repository version/release: 0.5.2, pre-1.0 and changing quickly;
- license: [Apache-2.0](https://github.com/vercel-labs/native/blob/main/LICENSE);
- implementation: roughly 78% Zig in the repository;
- default UI: SDK-drawn retained UI in real native windows, without a browser/WebView/JavaScript runtime;
- macOS renderer: Metal with native scroll physics;
- Windows/Linux: software renderer at this stage;
- optional WebViews are separate; CEF support is macOS-only in the current docs;
- a limited set of platform-owned Native Controls exists, but most widgets are custom-drawn rather than AppKit/WinUI/GTK widgets.

Therefore “native” means native executable/window/event integration and non-web default rendering. It does **not** mean every control is an OS-native widget. Primary project references: [README](https://github.com/vercel-labs/native#readme), [platform support](https://native-sdk.dev/platform-support), [native controls](https://native-sdk.dev/native-controls), and [web engines](https://native-sdk.dev/web-engines).

### Platform readiness relevant to SD-300

| Area | macOS | Windows | Linux |
|---|---|---|---|
| Core window/custom UI | Strongest path | Available, software-rendered | Available, software-rendered |
| IME/input edge cases | Better-developed | Real-hardware qualification still important | Compositor/distro matrix required |
| Tray | Available per current matrix | Available/verify packaging | Not currently equivalent |
| Packaging | `.app`/DMG helper | Early; no complete installer/signing story | Early/distribution-specific |
| Signing | none/ad-hoc/Developer ID modes | Not SDK-managed | Not equivalent |
| Web engine | system/CEF options vary | Limited current path | Limited current path |

Current packaging/signing references: [packaging](https://native-sdk.dev/packaging) and [signing](https://native-sdk.dev/packaging/signing). macOS minimum is 11 according to the current support material. Developer ID packaging can take an entitlements path; notarization remains a separate workflow. Windows installer/signing must be designed outside the SDK today.

### Recommended experiment boundary

Do not rewrite the monitor in Zig or duplicate collectors in GUI code. Use this sequence:

1. **Canonical Rust library/domain model** powers CLI and TUI.
2. **Versioned sidecar protocol first:** long-lived `sd300 agent --stdio` or a local socket streams NDJSON/CBOR observations and accepts narrow commands. Native can spawn and supervise it. Never spawn one shell command per tile/tick.
3. **Experimental Native GUI workspace member/artifact** consumes the protocol and proves window/input/packaging on all three OS families.
4. **Optional FFI later:** if latency/packaging justifies it, expose a small `cdylib`/`staticlib` C ABI with `#[repr(C)]`, opaque handles, explicit allocation/free functions, panic containment, and ABI version negotiation. Rust references: [linkage](https://doc.rust-lang.org/reference/linkage.html) and [FFI](https://doc.rust-lang.org/nomicon/ffi.html).
5. The GUI remains optional; missing GUI dependencies can never block CLI/TUI builds or releases.

The Native repository’s [system-monitor example](https://github.com/vercel-labs/native/tree/main/examples/system-monitor) shells out to commands such as `ps`, `vm_stat`, and `sysctl` on a timer. Treat it as a showcase, not the SD-300 architecture. The product needs one reusable collector core with long-lived native handles.

### Experiment acceptance gates

- builds and starts on macOS arm64/x86_64, Windows x86_64, and Linux x86_64/arm64 targets in scope;
- no WebView/CEF unless explicitly chosen;
- sidecar lifecycle, version mismatch, crash, backpressure, and reconnection tested;
- keyboard-only navigation, screen-reader semantics, scaling, high contrast, IME, and reduced motion tested;
- idle CPU/memory reasonable with live charts;
- code signing/notarization and Windows packaging have a credible independent plan;
- GUI can be removed without touching collector logic or TUI/CLI functionality.

### Local prerequisite note

Zig, the `native` CLI, CMake, Ninja, and pkg-config were not installed on this Mac. No SDK proof-of-concept was created because the task explicitly forbids the major implementation now and this borrowed machine should not be mutated unnecessarily.

---

## Different-Mac qualification matrix

This M2 laptop cannot establish universal behavior. Build fixtures and obtain physical runs for the following matrix:

| Mac class | Unique questions |
|---|---|
| Fanless Apple Silicon Air | `FNum=0` vs no SMC access; passive thermal pressure; opaque channel set |
| Base-chip fan laptop (this class) | One fan; M1/M2/M3/M4/M5 key casing/types; sleep/wake SMC stability |
| Pro/Max 14-inch and 16-inch laptops | Two/multiple fans, more P/E/GPU clusters, different DVFS tables, multiple sensor banks |
| Mac mini | Desktop power source, one/more fans, no battery/lid/internal display |
| Mac Studio / Ultra | many GPU cores, dual-die/Ultra topology, more fans/sensors/media engines |
| Apple Silicon Mac Pro | PCIe inventory, external cards/storage, multiple display paths, chassis fans |
| Intel MacBook | Intel frequencies/turbo, legacy SMC sensor keys, optional discrete GPU, T2 interactions |
| Intel desktop/Mac Pro | multi-socket/core topology, AMD GPU(s), PCIe, multiple drives/fans |
| eGPU-capable Intel Mac | multiple Metal devices, removable GPU, display association, AMD telemetry |
| Virtualized macOS guest | synthetic model/devices, no SMC/battery, reduced Metal, guest confidence |
| Rosetta process on Apple Silicon | API/ABI/private-library behavior from x86_64 process; helper architecture |
| External NVMe/SATA/USB storage | SMART passthrough, protocol bridges, removable/hot-plug, serial redaction |
| Multiple/HDR/high-refresh displays | mode lists, VRR, EDR/HDR, scaling/mirroring, GPU association |
| Enterprise/MDM Mac | TCC/PPPC policy, network filters, endpoint tools, command restrictions |
| Sandboxed/App Store build | private API rejection, entitlement constraints, reduced visibility |

For every run capture OS version/build, model identifier, SoC/Intel CPU, form factor, process architecture, distribution/signature context, privilege, source status, and sanitized capability registry. Do not compare raw private channel names across machines without model mapping/version evidence.

---

## Alienware implementation roadmap

### Phase 0 — correctness before breadth

1. Introduce `Observation<T>`, availability reasons, provenance, sensitivity, scope, and confidence.
2. Stop converting missing data to Good/Quiet/Integrated/Connected.
3. Separate workload, pressure, temperature, capacity, errors, wear, and health.
4. Correct ping semantics, local clock, driver scan state, APFS mapping, process ranking, and command execution.
5. Add a redacted `capabilities`/`snapshot` CLI path for fixture collection and support.

### Phase 1 — public macOS core

1. Create the actual `collectors/platform/macos` backend boundary.
2. Add native topology/Mach memory/CPU, `ProcessInfo`, power sources, Disk Arbitration/IOKit, Metal, CoreGraphics, CoreAudio, SystemConfiguration/Network, and CoreWLAN adapters.
3. Keep Apple commands only as bounded background/on-demand fallbacks.
4. Add typed device/topology models and notification-driven refresh.
5. Build parser/API fixtures from sanitized schemas, including absent/denied/empty-success/malformed cases.

### Phase 2 — enhanced Mac backend

1. IOHID temperature adapter with conservative validity filters and raw labels.
2. Read-only AppleSMC adapter with symbol/service/key/type/size feature detection and zero writes.
3. IOReport subscription adapter with a tiny allowlist of named metrics, unit validation, sample-window checks, and fallback.
4. IODeviceTree DVFS table parser with model-specific fixtures.
5. AGX/block/battery experimental fields behind an “experimental private telemetry” setting.

### Phase 3 — TUI/CLI depth

1. User Mode: concise pressure/condition explanations with no raw private claims.
2. Technician Mode: searchable/filterable/scrollable sensors, engines, devices, capabilities, and provenance.
3. Show `why unavailable` and `how to enable` without pushing users toward unnecessary privilege.
4. Add time-series validity/gap markers and source-change markers.
5. Export safe snapshots by default and sensitive snapshots only with explicit preview/confirmation.

### Phase 4 — optional privileged capture

1. Threat-model a signed helper.
2. Implement bounded `powermetrics` plist capture only if it adds metrics not reliably covered by IOReport/public APIs.
3. Test authorization, cancellation, update, signing, and failure paths.
4. Keep the main UI/collector useful with the helper absent.

### Phase 5 — experimental GUI

1. Stabilize the Rust stream protocol.
2. Build a Vercel Labs Native proof of concept as a separate package/artifact.
3. Qualify all target OSes and accessibility/input/packaging.
4. Decide whether it graduates only after the primary TUI/CLI data model is stable.

---

## Later Mac validation checklist

### Safe ordinary-user regression

- start from clean boot, then repeat after sleep/wake and user switch;
- capture idle, ordinary workload, and natural heavy workload without synthetic stress;
- verify all 38 direct IOHID services and current `sysinfo` subset;
- verify SMC fan count/keys/type/RPM at stopped/low/high natural fan states;
- validate IOReport delta units, state sums, energy plausibility, missing groups, and counter reset;
- compare public pressure to temperatures/fans without inferring a universal threshold;
- attach/detach power and sample charging/discharging/full sentinels;
- add/remove USB, Thunderbolt, audio, display, and storage devices;
- change Wi-Fi/VPN/interface state without recording identities;
- validate APFS graph across system/data/recovery/VM/external containers;
- test terminal widths 80×24, 100×30, and wide for both modes/all sections;
- verify no raw identifier enters logs, panic text, screenshots, fixtures, or exports.

### Signed-app/TCC matrix

- unsigned CLI in Terminal;
- Developer ID CLI;
- signed unsandboxed `.app`;
- sandboxed build if considered;
- no permissions granted;
- Location allowed/denied;
- camera/microphone explicit test allowed/denied;
- Full Disk Access absent (required baseline);
- optional helper authorized/denied/unavailable;
- private API behavior under hardened runtime and notarized packaging.

### Privileged supervised capture

Only on owned/supervised hardware, use a bounded sample similar to:

```bash
sudo /usr/bin/powermetrics \
  --sample-count 3 \
  --sample-rate 1000 \
  --samplers cpu_power,gpu_power,ane_power,thermal,battery \
  --format plist
```

Confirm the exact current help first; sampler availability varies. Redact process names/IDs and never persist the unfiltered plist.

### Fleet fixtures required

- at least one fanless Apple Silicon Mac;
- one Pro/Max laptop with multiple fans;
- one Apple Silicon desktop;
- one Intel/T2 laptop or desktop;
- Rosetta x86_64 execution;
- external SMART-capable SATA/NVMe bridge;
- multi-display/HDR/high-refresh configuration;
- a restricted enterprise/TCC context if that market matters.

---

## Reproducible probe recipes

These commands are listed as test recipes, not production architecture. Run individually, sanitize immediately, and do not commit raw output.

### Low-risk static/status probes

```bash
/usr/bin/sw_vers
/usr/bin/uname -m
/usr/sbin/sysctl hw.model hw.machine hw.ncpu hw.physicalcpu hw.logicalcpu hw.memsize
/usr/sbin/sysctl -a | /usr/bin/grep '^hw\.perflevel'
/usr/sbin/sysctl kern.hv_support
/usr/sbin/sysctl -n sysctl.proc_translated
/usr/bin/pmset -g batt
/usr/bin/pmset -g therm
/bin/launchctl print system/com.apple.bluetoothd
/bin/launchctl print system/com.apple.audio.coreaudiod
```

Treat an empty value as unavailable even when `sysctl` exits 0. Do not pass a long mixed key list unless partial-success semantics are handled; one unknown key can make the command exit 1 after printing valid earlier keys.

### Structured storage probes

```bash
/usr/sbin/diskutil list -plist
/usr/sbin/diskutil info -plist disk0
/usr/sbin/diskutil apfs list -plist
```

Parse only allowlisted fields. `diskutil info /` can describe the sealed boot snapshot and report `Encrypted: No` while the APFS System/Data volume set is FileVault-encrypted; volume role/topology matters.

On macOS 26, inspect the optional
`SMARTDeviceSpecificKeysMayVaryNotGuaranteed` dictionary only through the typed,
sanitized rules above. Do not dump its machine-lifetime counters into a default
support bundle.

### Allowlisted profiler types

Invoke exactly one type at a time:

```bash
/usr/sbin/system_profiler -json SPHardwareDataType
/usr/sbin/system_profiler -json SPDisplaysDataType
/usr/sbin/system_profiler -json SPNVMeDataType
/usr/sbin/system_profiler -json SPPowerDataType
/usr/sbin/system_profiler -json SPUSBHostDataType
/usr/sbin/system_profiler -json SPThunderboltDataType
/usr/sbin/system_profiler -json SPAudioDataType
/usr/sbin/system_profiler -json SPCameraDataType
/usr/sbin/system_profiler -json SPBluetoothDataType  # controller-only allowlist
/usr/sbin/system_profiler -json SPAirPortDataType    # radio/link-only allowlist
```

Use the absolute executable, cleared environment plus `LC_ALL=C`/`LANG=C`, null
stdin, the per-type timeout/caps documented in the payload section, and
concurrent bounded pipe draining. Do not run “all data types” as a normal
diagnostic capture. A data type can exit 0 with no payload, so validate its
top-level array and required fields. Never persist raw Bluetooth/Wi-Fi/device
subtrees even temporarily.

### Temporary native probes used

- public Swift: `ProcessInfo`, Metal, CoreGraphics, AppKit, CoreWLAN;
- IOHID event-system temperature services;
- read-only AppleSMC key reads;
- IOReport inventory/subscription/delta sampling;
- I/O Registry property allowlists for AGX, block-driver, battery, and engine presence;
- a temporary Rust path dependency to exercise the current SD-300 library.

All temporary source/binaries stayed outside the repository. No raw capture was added to Git.

---

## Source map

### Apple public documentation

Identity/CPU/memory:

- [`sysctlbyname` capability discovery](https://developer.apple.com/documentation/kernel/1387446-sysctlbyname/determining_system_capabilities)
- [`ProcessInfo`](https://developer.apple.com/documentation/foundation/processinfo)
- [`systemUptime`](https://developer.apple.com/documentation/foundation/processinfo/systemuptime)
- [`host_processor_info`](https://developer.apple.com/documentation/kernel/1502854-host_processor_info)
- [`host_statistics64`](https://developer.apple.com/documentation/kernel/1502863-host_statistics64)
- [Dispatch memory-pressure source](https://developer.apple.com/documentation/dispatch/dispatchsourcememorypressure)
- [`DISPATCH_SOURCE_TYPE_MEMORYPRESSURE`](https://developer.apple.com/documentation/dispatch/dispatch_source_type_memorypressure)

Hardware/power/storage:

- [IOKit](https://developer.apple.com/documentation/iokit)
- [IOHID manager](https://developer.apple.com/documentation/iokit/iohidmanager_h)
- [IOPowerSources](https://developer.apple.com/documentation/iokit/iopowersources_h)
- [`IOPMCopyCPUPowerStatus`](https://developer.apple.com/documentation/iokit/1557079-iopmcopycpupowerstatus)
- [`IOPMGetThermalWarningLevel`](https://developer.apple.com/documentation/iokit/1557103-iopmgetthermalwarninglevel)
- [Disk Arbitration constants](https://developer.apple.com/documentation/diskarbitration/diskarbitration-constants)
- [NVM Express Base Specification 2.0a SMART/Health fields](https://nvmexpress.org/wp-content/uploads/NVMe-NVM-Express-2.0a-2021.07.26-Ratified.pdf)
- [NVM Express NVMe-CLI health-monitoring overview](https://nvmexpress.org/open-source-nvme-management-utility-nvme-command-line-interface-nvme-cli/)

Graphics/display/audio/network:

- [`MTLDevice`](https://developer.apple.com/documentation/metal/mtldevice)
- [Metal GPU counters](https://developer.apple.com/documentation/metal/gpu-counters-and-counter-sample-buffers)
- [Quartz Display Services](https://developer.apple.com/documentation/CoreGraphics/quartz-display-services)
- [Core Audio functions](https://developer.apple.com/documentation/coreaudio/core-audio-functions)
- [CoreWLAN](https://developer.apple.com/documentation/CoreWLAN)
- [Network framework path monitoring](https://developer.apple.com/documentation/network/nwpathmonitor)
- [SystemConfiguration](https://developer.apple.com/documentation/systemconfiguration/scdynamicstore-gb2)

Security/distribution:

- [App Review Guideline 2.5.1 public-API requirement](https://developer.apple.com/app-store/review/guidelines/)
- [App Sandbox](https://developer.apple.com/documentation/security/app-sandbox)
- [Hardened Runtime](https://developer.apple.com/documentation/security/hardened-runtime)
- [Notarizing macOS software](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution)
- [Customizing notarization](https://developer.apple.com/documentation/security/customizing-the-notarization-workflow)
- [Service Management](https://developer.apple.com/documentation/servicemanagement)
- [Privacy manifest](https://developer.apple.com/documentation/bundleresources/adding-a-privacy-manifest-to-your-app-or-third-party-sdk)

### Private-interface implementation references

Apple publishes no supported consumer documentation for AppleSMC, IOReport.framework, AGX `PerformanceStatistics`, AppleSmartBattery raw dictionaries, NVMeSMARTLib, DisplayServices, or CoreDisplay. These references help understand implementations but do not create a compatibility promise:

- [macmon’s pinned IOReport/SMC source](https://github.com/vladkens/macmon/blob/337350e18a46e41a060c6c3f9d75793568e16873/src/sources.rs) and its [IOReport explanation](https://vladkens.cc/powermetrics-macos/) for Apple Silicon monitoring patterns;
- [OSHI’s IOReport JNA mapping](https://www.oshi.ooo/xref/oshi/jna/platform/mac/IOReport.html) as an independent ABI cross-check;
- [Chromium Apple Silicon sensor implementation](https://chromium.googlesource.com/chromium/src/+/112.0.5615.165/components/power_metrics/m1_sensors_mac.mm) for the IOHID route;
- [pinned macOS SMC fan protocol research](https://github.com/agoodkind/macos-smc-fan/blob/f95560210ca60e7d7f579b72c2399b93a3e11dfd/docs/research.md) for model-generation key differences and result codes.

### Host/product references

- Apple’s [`Mac14,7` model mapping](https://support.apple.com/108052)
- [MacBook Pro 13-inch M2 technical specifications](https://support.apple.com/111869)
- [Apple repair manual](https://support.apple.com/100511)
- [Model-specific fan replacement guide](https://www.ifixit.com/Guide/MacBook+Pro+13-Inch+2022+(M2)+Fan+Replacement/157929)

### Vercel Labs Native and Rust integration

- [Native repository/README](https://github.com/vercel-labs/native#readme)
- [Native v0.5.2 release snapshot](https://github.com/vercel-labs/native/releases/tag/v0.5.2)
- [Native platform support](https://native-sdk.dev/platform-support)
- [Native controls](https://native-sdk.dev/native-controls)
- [Native web engines](https://native-sdk.dev/web-engines)
- [Native packaging](https://native-sdk.dev/packaging)
- [Native signing](https://native-sdk.dev/packaging/signing)
- [Native system-monitor example](https://github.com/vercel-labs/native/tree/main/examples/system-monitor)
- [Rust linkage](https://doc.rust-lang.org/reference/linkage.html)
- [Rust FFI guidance](https://doc.rust-lang.org/nomicon/ffi.html)

---

## Evidence limits, open questions, and confidence

### Not tested deliberately

- interactive sudo or any permission prompt;
- fan/SMC writes;
- synthetic thermal stress;
- battery discharge, sleep/wake, lid transitions, or destructive/offline tests;
- external device unplugging;
- Intel or another Apple Silicon model;
- App Sandbox/App Store review;
- Developer ID signing/notarization for a GUI;
- third-party `smartctl` installation;
- a Vercel Native proof of concept.

### Highest-value open questions

1. Do AppleSMC read-only keys work under hardened runtime/sandbox and across M1 through current generations?
2. Which private IOHID labels map consistently enough to CPU/GPU/SoC/storage on each model family?
3. Which IOReport channels/units survive OS updates, sleep, and counter resets?
4. Can public APIs cover enough energy/frequency detail to avoid a private backend in a distributable build?
5. How should an optional signed helper be installed/updated/authorized without expanding attack surface?
6. On which macOS/model versions does `SMARTDeviceSpecificKeysMayVaryNotGuaranteed` exist, and do its limb/unit semantics match this macOS 26 M2 observation?
7. Does Vercel Native mature its Windows/Linux renderer, accessibility, IME, tray, packaging, and signing paths enough for production?

### Confidence by claim class

| Claim | Confidence |
|---|---|
| Exact host identity/topology and public API observations | High |
| Current SD-300 defects and live behavior on this host | High |
| 33 current-dependency sensor series | High |
| Direct IOHID 38-service access and timing here | High |
| AppleSMC fan values/access here | High |
| IOReport inventory/delta/energy/residency here | High |
| `diskutil` private NVMe SMART dictionary and exact values here | High on this host; low cross-version contract confidence |
| Private API units/semantics across macOS/model generations | Medium to low until fleet-qualified |
| Root `powermetrics` availability/privilege boundary here | High |
| General future macOS architecture | High as a risk-reducing design; runtime details remain testable hypotheses |
| Vercel Native current feature assessment | Medium-high, but fast-moving pre-1.0 status requires re-check before implementation |

### Final reasoning conclusion

The best explanation of all observations is the **layered-access hypothesis**: macOS offers a broad stable public baseline, a surprisingly rich unprivileged private tier, an optional root tier, and irreducible gaps. The convenience-sample warning also stands: this M2 MacBook Pro is unusually valuable evidence, but its labels, SMC keys, fan count, DVFS tables, and IOReport channels cannot be universalized.

The practical definition of an exhaustive SD-300 is therefore:

> Discover every supported capability, collect every safely justified observation, preserve source/scope/unit/age/access/confidence, explain every unavailable field, and never invent a health claim from missing data.

That design can be built primarily on the Alienware now. This report defines which parts are safe to implement from fixtures and which must come back to real Mac hardware before release.
