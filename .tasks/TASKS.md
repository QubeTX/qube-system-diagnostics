# Tasks

## Backlog

## To-Do

- [ ] **Expand cross-platform diagnostic parity** - Windows is live-proven; hosted macOS/Linux ABI and hardware evidence remains (needs #obs) (ms #v20) #dpx
- [ ] **Ship Windows Global and Corporate MSI/EXE lifecycle** - four installer channels with verified handoff and takeover (needs #upd) (ms #v20) #wix
- [ ] **Ship direct signed macOS PKG lifecycle** - PKG-first install/update with immutable-client compatibility bridge (needs #upd) (ms #v20) #pkg
- [ ] **Qualify Linux diagnostics and lifecycle** - shell/Cargo ownership plus native hardware evidence (needs #obs, #upd) (ms #v20) #lnx
- [ ] **Harden versionless release automation** - exact-tag verification behind stable latest assets (needs #dpx, #wix, #pkg, #lnx) (ms #v20) #rel
- [ ] **Publish website v2 content and re-list SD-300** - list immediately after ND-300 only after verified public release (needs #ga2) (ms #v20) #web

## Active

- [ ] **Qualify and release SD-300 v2.0.0** - local implementation and Apple credential provisioning are green; hosted matrices remain (needs #rel) (ms #v20) #ga2
  - [x] Qualify the release Windows binary on Alienware hardware
  - [x] Pass Rust, workflow, package, cross-target, and website local gates
  - [x] Configure the seven Apple certificate/notary repository secrets (three non-secret variables are copied)
  - [ ] Run Windows, Intel Mac, Apple Silicon, and Linux hosted qualification
  - [ ] Verify fresh public bytes, commands, crate, and lifecycle before website merge

## Done

- [x] **Qualify Alienware Windows diagnostics** - live accuracy audit and 18-view TUI regression suite on Windows 11 (ms #v20) #wqa
- [x] **Build explicit observation and capability model** - unavailable, denied, unsupported, contradictory, and failed telemetry is explicit (ms #v20) #obs
- [x] **Build channel-preserving install, update, and uninstall engine** - exact-origin updates, authoritative fresh takeover, and fail-closed ownership (ms #v20) #upd
