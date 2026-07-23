# Tasks

## Backlog
- [ ] **Run the released-bytes two-hour soak and capture frame/input percentiles** - TASK FOR CODEX; unattended, machine-quarantined two-hour Processes soak plus formal frame-p95/input-p95 evidence on the public v3 bytes, with exit-attribution awareness; replaces the pre-release soak gate the operator waived on 2026-07-22 (needs #qv3) (ms #v3n) #sok
- [ ] **Run the extensive post-release testing and performance sweep** - TASK FOR CODEX; everything waived from the v3.0.0 release under the operator's functional-bar directive: exhaustive GUI automation across all sections/modes/keyboard/scaling/exports/unavailable states, published-v2 PTY replay on hosted targets, physical interaction regression sweep (scroll granularity, tray and minimize lifecycle), formal foreground/hidden budget re-proof, and varied-load performance regression checks; feeds patch releases (needs #qv3) (ms #v3n) #ext
- [ ] **Run the post-release hardening sweep** - TASK FOR CODEX; deferred robustness items from the release reviews: uninstall-fix trio, engine staleness/threading/panic containment, GUI and installer polish, documented quirks (needs #qv3) (ms #v3n) #hrd
- [ ] **Resolve Makira embedding-license evidence** - obtain vendor documentation for the purchased face or replace with an open font in a patch release; operator-led #mkl
- [ ] **Run physical macOS acceptance on real hardware** - installed PKG lifecycle, GUI/status-item behavior, and notarization experience when Mac access returns #mac
- [ ] **Improve GUI screen-reader accessibility beyond the named canvas** - track Native SDK accessibility-tree support; TUI remains the documented fallback #acc

## To-Do

## Active

## Done
- [x] **Release-workflow hygiene: skip cleanly on post-release same-version main pushes** - PUBLIC as v3.1.1 2026-07-23 02:10 UTC (tag 4c612be). Refined release.yml source-check so an already-fully-published version whose main source commit differs from its immutable tag SKIPS deploy cleanly (green) instead of failing; guard still fires on real deploy-path conflicts (fresh version with a conflicting tag, or crate-published/release-missing repair from the wrong commit). All four branches simulated before shipping; documented as a warn-and-investigate note in AGENTS.md. Shipped the v3.1.0 post-release docs (ADRs 0004/0005, README pass, gui-engine testing-doc fix). Verified public: crate installs to `sd300 3.1.1`, release promoted to latest (59 assets, 26 sidecars, SBOM), `gh attestation verify` returns two SLSA attestations bound to 4c612be. Release recovery note: a mis-timed board push during the release split the branch head from the draft target (eba3195 vs 4c612be) and I wrongly canceled two workflow_run producers (they branch-head-associate) — recovered by deleting the draft and re-running Release from 4c612be so all provenance is single-commit; lessons recorded in operator memory (done 2026-07-23) (ms #v3n) (agent: opus)
- [x] **Add safe in-app and tray-driven updates** - PUBLIC as v3.1.0 2026-07-23 00:25 UTC (merge 2b4c9a5, PR #5): Settings/tray "Update now" spawns the installed CLI as a detached coordinator running the existing owner-preserving transaction; hidden --relaunch-gui reopens only after success; architecture committed in ADR 0005. Crate live (`cargo install tr300-tui --version 3.1.0` -> sd300 3.1.0), release promoted to latest with 59 assets + SHA-256 sidecars + SPDX SBOM, `gh attestation verify` returns two SLSA attestations bound to 2b4c9a5. Carried the two post-release fixes (sidebar label, crate convergence poll) (done 2026-07-23) (ms #v3n) #giu
  - [x] Implement the update trigger in settings and tray with a detached engine-spawned coordinator
  - [x] Add the hidden CLI relaunch flag gated to successful transactions
  - [x] Validate: Rust and native tests, strict checks, local installed-copy coordinator exercise
  - [x] Bump product version surfaces to 3.1.0 with lockstep changelogs
  - [x] Merge, release, and verify public bytes (crate, latest release, attestations, stable URLs)
- [x] **Qualify and release SD-300 v3.0.0 on all six existing targets** - PUBLIC 2026-07-22 15:02 UTC with 59 assets, live crate, three verified attestations, and physical Windows acceptance on the released MSI; stable versionless installer URLs keep existing website commands working unchanged (done 2026-07-22) (ms #v3n) #qv3
  - [x] Re-prove foreground and hidden performance budgets on the fixed build
  - [x] Design and implement the app, taskbar, tray, and installer icons with Quiver arrow-1.1-max
  - [x] Run a bounded post-fix performance sanity sample (formal budget re-proof in Backlog #sok)
  - [x] Merge PR #4 to main (4e17c41); release workflow driving to full green
  - [x] Publish v3.0.0 (release public 2026-07-22 15:02 UTC, 59 assets, crate live, attestations green)
  - [x] Verify fresh public bytes, Cargo install, and physical Windows acceptance
- [x] **Extend every installer, updater, repair, and uninstall path with the GUI companion** - preserve all non-Cargo owners and qualify the explicit two-step Cargo migration; hosted run 29917852561 green end-to-end (done 2026-07-22) (ms #v3n) #cpl
  - [x] Land the reviewed receipt-parent cleanup fix with lockstep changelogs and ADR
  - [x] Pass exact-head hosted Windows Native Installers qualification with sibling preservation
- [x] **Build the complete QubeTX-native diagnostic GUI** - all nine sections, both modes, settings, exports, tray behavior, scroll fix with evidence; heavy testing waived to #sok/#ext per operator (done 2026-07-22) (ms #v3n) #gux
  - [x] Lock the approved Warm Carbon design and bundled font hierarchy
  - [x] Make all nine navigation destinations functional with bounded live projections
  - [x] Complete audience modes, settings, exports, sorting/filtering, and unavailable-state parity
  - [x] Attribute the early clean soak exit to its graceful-quit source
  - [x] Attribute the scroll lag with a warmed-state damage benchmark and record ADR 0002
  - [x] Reproduce and eliminate the severe minute-old scroll/input lag on scrollable sections
  - [x] Qualify keyboard, scaling, tray/autostart interaction; sustained-performance evidence moved to Backlog #sok
- [x] **Prove the pinned Native SDK, shared Rust engine, and Windows MSI vertical slice** - v2 baselines, native GUI, reproducible dependencies, Corporate MSI proof; PTY replay waived to backlog per operator (done 2026-07-22) (ms #v3n) #nsp
  - [x] Capture immutable v2.0.6 CLI/TUI compatibility fixtures
  - [x] Scaffold the pinned native-rendered Zig application without a local-path dependency
  - [x] Export and dynamically load the bounded Rust monitoring ABI
  - [x] Exercise the GUI with Native SDK automation and performance profiling
  - [x] Build, install, launch, verify, and uninstall a Corporate MSI candidate
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
