<!-- tasks-bootstrap: complete -->
> Secrets: never stored here or in memory/. See .tasks/secure/ (gitignored), or env/keychain.

## Me

| Item | Detail |
|---|---|
| Operator | Emmett Shaughnessy |
| Environment | Alienware Windows workstation; temporary Mac access is no longer available |

## People

| Person | Context |
|---|---|

## Terms

| Term | Meaning |
|---|---|
| SD-300 | This repository's cross-platform Rust system diagnostic TUI; command `sd300`, crate `tr300-tui` |
| TR-300 | QubeTX machine-report product used as a lifecycle and packaging reference |
| ND-300 | QubeTX network-diagnostics product used as a lifecycle and packaging reference |
| Corporate | Per-user Windows installer channel that does not require administrator installation |
| Global | Per-machine Windows installer channel installed under Program Files |

## Projects

| Project | Status |
|---|---|
| SD-300 v2.0.2 | Complete: Windows accuracy, full updater lifecycle, installers, parity, release, offline bundle, and website rollout |
| SD-300 v3.0.0 | Released and publicly verified 2026-07-22 15:02 UTC (PR #4 → main, 59 assets, live crate, attestations, physical Windows MSI acceptance) |
| SD-300 v3.1.0 | In-app/tray updates (#giu) on `codex/sd300-giu-in-app-updates` PR #5 (2026-07-22): coordinator architecture per ADR 0005; all local gates green; merge → main release → public-byte verify, then post-release docs push (ADRs 0004/0005, README pass, memory) and branch cleanup |

## Preferences

| Preference | Detail |
|---|---|
| Install defaults | Advertise `irm` on Windows and `curl | sh` on macOS/Linux; native installers remain options |
| Update ownership | `update` preserves the proven channel; a fresh official install becomes the latest user intent |
| Public artifacts | Stable versionless names and latest URLs; exact versions remain internal for integrity |
| Evidence | Distinguish live hardware, hosted native, fixture, cross-compile, and research proof |
| Board | Git-track the task board and keep status, verification, and activity current |
| Release honesty | A build or CI pass is not physical UI, installer, UAC, long-soak, signing, license, provenance, or public-byte acceptance; keep those claims separate |
