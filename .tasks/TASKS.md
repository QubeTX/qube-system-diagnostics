# Tasks

## Backlog

## To-Do

- [ ] **Qualify and release SD-300 v3.0.0 on all six existing targets** - pass compatibility, lifecycle, performance, provenance, hosted, physical Windows, and public-byte gates across Windows x86-64, macOS x86-64/ARM64, Linux GNU x86-64/ARM64, and Linux musl x86-64; probe additional SDK architectures without silently expanding the owned release contract (needs #gux, #cpl) (ms #v3n) #qv3

## Active

- [ ] **Prove the pinned Native SDK, shared Rust engine, and Windows MSI vertical slice** - establish v2 compatibility baselines, a native CPU/memory GUI, reproducible dependencies, and an installed Corporate MSI smoke before full UI work (ms #v3n) #nsp
  - [ ] Capture immutable v2.0.6 CLI/TUI compatibility fixtures
  - [ ] Scaffold the pinned native-rendered Zig application without a local-path dependency
  - [ ] Export and dynamically load the bounded Rust monitoring ABI
  - [ ] Exercise the GUI with Native SDK automation and performance profiling
  - [x] Build, install, launch, verify, and uninstall a Corporate MSI candidate
- [ ] **Build the complete QubeTX-native diagnostic GUI** - implement all nine sections, both modes, settings, exports, accessibility, and supported tray behavior (needs #nsp) (ms #v3n) #gux
  - [x] Lock the approved Warm Carbon design and bundled font hierarchy
  - [x] Make all nine navigation destinations functional with bounded live projections
  - [ ] Complete audience modes, settings, exports, sorting/filtering, and unavailable-state parity
  - [ ] Qualify keyboard, scaling, tray/autostart, and sustained renderer performance
- [ ] **Extend every installer, updater, repair, and uninstall path with the GUI companion** - preserve all non-Cargo owners and qualify the explicit two-step Cargo migration (needs #nsp) (ms #v3n) #cpl

## Done

- [x] **Align driver and thermal health reporting** - Alienware driver parity, truthful thermal provider coverage, and lifecycle fix-forward release are live in v2.0.6 #dth
  - [x] Use authoritative Windows PnP problem-code precedence
  - [x] Show every genuine driver issue counted by the overview
  - [x] Merge available GPU and hardware-monitor thermal readings with provenance
  - [x] Prove snapshot and TUI parity on Alienware plus hosted targets
- [x] **Prove complete CLI uninstall and publish the command** - `sd300 uninstall` removes every proven owner and is live on the SD-300 website #unx
  - [x] Verify public managed PowerShell update, uninstall, and reinstall on Alienware
  - [x] Exercise Corporate MSI CLI uninstall on Alienware
  - [x] Make hosted Windows qualification invoke CLI uninstall for all four native channels
  - [x] Publish and verify the website command surfaces
- [x] **Qualify and release SD-300 v2** - v2.0.2 is the qualified fix-forward release with all hosted/public gates green (needs #rel) (ms #v20) #ga2
  - [x] Qualify the release Windows binary on Alienware hardware
  - [x] Pass Rust, workflow, package, cross-target, and website local gates
  - [x] Configure the seven Apple certificate/notary repository secrets (three non-secret variables are copied)
  - [x] Run Windows, Intel Mac, Apple Silicon, and Linux hosted qualification
  - [x] Verify fresh public bytes, commands, crate, and lifecycle before website merge
- [x] **Publish website v2 content and re-list SD-300** - listed immediately after ND-300 after verified public release (needs #ga2) (ms #v20) #web
- [x] **Harden versionless release automation** - exact-tag verification behind stable latest assets (needs #dpx, #wix, #pkg, #lnx) (ms #v20) #rel
- [x] **Qualify Linux diagnostics and lifecycle** - shell/Cargo ownership plus native and fixture evidence (needs #obs, #upd) (ms #v20) #lnx
- [x] **Ship direct signed macOS PKG lifecycle** - PKG-first install/update with immutable-client compatibility bridge (needs #upd) (ms #v20) #pkg
- [x] **Ship Windows Global and Corporate MSI/EXE lifecycle** - four installer channels with verified handoff and takeover (needs #upd) (ms #v20) #wix
- [x] **Expand cross-platform diagnostic parity** - Windows live proof plus hosted native platform qualification (needs #obs) (ms #v20) #dpx
- [x] **Qualify Alienware Windows diagnostics** - live accuracy audit and 18-view TUI regression suite on Windows 11 (ms #v20) #wqa
- [x] **Build explicit observation and capability model** - unavailable, denied, unsupported, contradictory, and failed telemetry is explicit (ms #v20) #obs
- [x] **Build channel-preserving install, update, and uninstall engine** - exact-origin updates, authoritative fresh takeover, and fail-closed ownership (ms #v20) #upd
