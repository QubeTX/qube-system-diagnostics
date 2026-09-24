# SD-300 4.0.2 installer license verification

Date: 2026-09-24

## Scope and identity

Replace the WiX placeholder with the existing PolyForm Noncommercial 1.0.0
LICENSE.md in Global and Corporate MSI dialogs. License terms and acceptance
navigation are unchanged. PR #14 merged as
`abf55bd47a99ae33818d46d9c844d96353379541`; immutable tag `v4.0.2` resolves to
that exact commit. The release was published at 2026-09-24 23:27:10 UTC with
59 assets. GitHub latest and crates.io both identify 4.0.2.

## Qualification

- Candidate CI [36067187928](https://github.com/QubeTX/qube-system-diagnostics/actions/runs/36067187928)
  and Windows lifecycle [36067190730](https://github.com/QubeTX/qube-system-diagnostics/actions/runs/36067190730) passed.
- Merged-source CI [36069849623](https://github.com/QubeTX/qube-system-diagnostics/actions/runs/36069849623) passed.
- Production Release [36069849686](https://github.com/QubeTX/qube-system-diagnostics/actions/runs/36069849686),
  Windows [36070510863](https://github.com/QubeTX/qube-system-diagnostics/actions/runs/36070510863),
  macOS [36070510904](https://github.com/QubeTX/qube-system-diagnostics/actions/runs/36070510904),
  and Linux [36070510894](https://github.com/QubeTX/qube-system-diagnostics/actions/runs/36070510894) passed.
- [ADR 0021](../../adr/0021-eula-only-release-performance-exception.md) records the operator's
  4.0.2-only performance exception. Reports retain failed timing verdicts;
  the separate release acceptance requires complete functional evidence and
  clean shutdown. No performance improvement or benchmark pass is claimed.

Final [Qualify and Publish run 36072546804](https://github.com/QubeTX/qube-system-diagnostics/actions/runs/36072546804) passed, including public immutable Cargo migration, managed lifecycle, checksums and attestation verification.

## Public MSI readback

Downloaded the public exact-tag Global, Corporate and compatibility MSI files.
Each reports ProductVersion 4.0.2, matches its SHA-256 sidecar, and passes
`gh attestation verify -R QubeTX/qube-system-diagnostics`. The public manifest
identifies the version, tag and merged SHA above.

`scripts/check-msi-license.ps1` reads the actual MSI Control table and verifies
LicenseAgreementDlg.LicenseText against the complete generated RTF. All three
public files passed; Windows RichEdit rendered 4357 characters. A separate
comparison established every rendered word matches LICENSE.md, and the checker
correctly rejected a previous placeholder MSI.

| Asset | SHA-256 |
|---|---|
| sd300-windows-x64-global.msi | 691dccb0e96fb211126db00a45eb294dd5757c57adaed0623c4d1f5bb502f39c |
| sd300-windows-x64-corporate.msi | 2224dd15c7b5bef2036c576b5d9e03fd801f6f9b6022f8c903b2b4b53aa3ccc8 |
| tr300-tui-x86_64-pc-windows-msvc.msi | 691dccb0e96fb211126db00a45eb294dd5757c57adaed0623c4d1f5bb502f39c |

The versionless latest-download routes for Global and Corporate were downloaded
separately and match the verified exact-tag bytes. Local evidence is under
`target/eula-public-4.0.2/`. This is MSI database and native rich-text rendering
proof, not a claim of manual visual acceptance of every wizard page.
