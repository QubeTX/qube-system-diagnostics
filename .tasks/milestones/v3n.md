TT;DR: Deliver SD-300 v3.0.0 as the existing Rust CLI/TUI plus a Vercel Native SDK desktop companion on every current release target. The work is active on a feature branch and must prove a Windows MSI vertical slice before any merge to main.

## Why
The operator directly requested a richer native GUI containing all SD-300 diagnostics while retaining the established terminal product and lifecycle. Native SDK was selected over Electron after research and disposable tests because its retained native renderer avoids a browser runtime and demonstrated a small idle footprint.

## Scope
Includes the Rust dynamic engine ABI, native GUI, QubeTX design translation, settings, supported tray behavior, all current release targets, additive installation/update/uninstall payloads, the explicit two-step Cargo migration, provenance, performance, compatibility fixtures, and release documentation. It excludes changing the existing TUI experience, silently dropping a target, a custom Linux tray bridge, automatic lifecycle actions inside the GUI, and merging before the requested MSI checkpoint is reviewed.

## Status
0/4 child tasks complete. The vertical-slice and complete-GUI tasks are active on `codex/sd300-v3-native-gui`; the visual system and nine live diagnostic destinations are implemented, and the requested physical Corporate MSI vertical slice now passes including a running-GUI CLI uninstall. Long-run performance, complete parity/settings, composite lifecycle work, all-target packaging, and release qualification remain open. The board uses tasks plugin 1.0.1 and the app is pinned to Native SDK 0.5.4 with Zig 0.16.0. One-second collection remains locked; software presentation consumes the latest bounded sample independently so rendering cannot create collector backlog. A clean, hash-guarded patched-SDK build now measures 1.92% of one logical core on Processes and passes the short foreground average gate, with the mandatory 15-minute/hidden/two-hour gates still open. The owned release matrix remains the approved six current SD-300 targets; Windows ARM64 and Linux-musl ARM64 are feasibility probes because Native SDK distributes those toolchains, not advertised products without full lifecycle proof.

## Completed

## Activity
- 2026-07-20 21:26 — created from the operator-approved implementation plan; feature-branch and Corporate MSI checkpoint added (agent: codex)
- 2026-07-21 00:18 — operator approved the real Warm Carbon render; all nine sections now consume shared collector projections, with 12/12 native tests and live Alienware interaction evidence recorded (agent: codex)
- 2026-07-21 01:30 — isolated the Native SDK Windows software-present bottleneck, preserved the one-second live-data requirement, recorded staged A/B measurements and the page-level information architecture, and kept foreground performance explicitly release-blocking (agent: codex)
- 2026-07-21 03:15 — separated one-second collection from eight-second latest-only presentation, measured production Overview at 0.93% and Processes at 2.63% of one logical core, and confirmed that the approved six-target matrix covers Windows x86-64, both macOS architectures, Linux GNU x86-64/ARM64, and Linux musl x86-64; two additional SDK architectures remain feasibility probes (agent: codex)
- 2026-07-21 04:08 — passed the physical Corporate MSI feature-branch checkpoint from a real Cargo-owned v2.0.6 state, including installed CLI-to-GUI launch and graceful running-GUI uninstall with no owned residue; kept merge/release blocked on the remaining parity, lifecycle, target, and sustained-performance gates (agent: codex)
