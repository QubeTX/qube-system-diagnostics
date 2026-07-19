TT;DR: Prove what SD-300 reports correctly on this Alienware and identify every Windows-specific omission or misleading state. Baseline Rust gates passed earlier, but live program and collector accuracy still require direct qualification.

## Why
Direct operator order. Previous work concentrated on macOS research; Rust portability does not prove Windows hardware coverage or semantic accuracy.

## Plan
Capture sanitized Windows ground truth using CIM/WMI, PDH, SetupAPI, storage, display, battery, GPU, event-log, and vendor-neutral sources. Run collectors and the TUI, compare every section, identify unavailable/permission-gated facts, then add fixtures and tests for reproducible regressions.

## Impact
This establishes the Windows truth baseline that drives collector changes. It must not expose serial numbers or infer health from missing sensor data.

## Acceptance
Every current collector and TUI section has a documented Alienware result, independent comparison, known limitations, and repeatable regression coverage for confirmed defects.

## Verification
- [x] Sanitized independent hardware inventory captured
- [x] Every TUI section exercised without crash or terminal corruption
- [x] Collector values compared with independent Windows sources
- [x] Confirmed defects represented by automated fixture or unit tests
- [x] Findings and exact resume point recorded on the board

## Status
Done locally. The final v2 release binary passed a redacted live snapshot, capability audit, independent hardware comparisons, and all 18 section/mode TestBackend renders at 80x24.

## Activity
- 2026-07-18 14:45 - created and moved directly to Active from the operator's original order.
- 2026-07-18 15:23 - captured independent Windows 11 inventory: Alienware m16 R2, Core Ultra 7 155H (16C/22T), 32 GiB across two DDR5-5600 DIMMs, Intel Arc plus RTX 4070 Laptop GPU, Samsung PM9A1 NVMe, two displays, active Intel Wi-Fi, and battery/provider states. Serial numbers and MAC addresses were excluded.
- 2026-07-18 15:23 - confirmed four accuracy risks before code changes: multi-GPU omission, null/error fan telemetry becoming false 0 RPM, missing storage reliability data lacking an explicit unavailable state, and WMI Degraded PnP status existing alongside SetupAPI problem code 0.
- 2026-07-18 16:31 - final release binary matched 16C/22T CPU topology, 31.5 GiB across two DIMMs, Intel Arc plus RTX 4070, two displays, 866.7 Mbps Wi-Fi, and four degraded PnP devices. Fan RPM remained explicitly unavailable; 23 capabilities and two warnings were emitted without default identifier leakage.
