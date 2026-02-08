# Changelog

All notable changes to SD-300 will be documented in this file.

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
