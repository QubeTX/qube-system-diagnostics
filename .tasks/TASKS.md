# Tasks

## Backlog

## To-Do

## Active

## Done

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
