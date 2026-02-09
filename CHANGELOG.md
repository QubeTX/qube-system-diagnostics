# Changelog

All notable changes to SD-300 will be documented in this file.

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
