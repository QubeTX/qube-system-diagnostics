# SD-300 4.0.0 public release verification

Recorded 2026-09-23 (Central time). This records public delivery and the defects
found during the final installed-copy check; it does not erase those failures.

## Published identity and native qualification

- [Product PR #10](https://github.com/QubeTX/qube-system-diagnostics/pull/10)
  merged as `170f70f01145e2ad04a6442df7e8d68bbf748fd1`.
- [SD-300 4.0.0](https://github.com/QubeTX/qube-system-diagnostics/releases/tag/v4.0.0)
  became public at 2026-09-24 00:09:50 UTC with 59 assets. Its immutable tag and
  release manifest identify that same commit; `tr300-tui` 4.0.0 is published
  and unyanked in the crates.io sparse index.
- [Merged-source CI](https://github.com/QubeTX/qube-system-diagnostics/actions/runs/35934278681)
  and [Release](https://github.com/QubeTX/qube-system-diagnostics/actions/runs/35934279035)
  passed. Native installers passed on
  [Windows](https://github.com/QubeTX/qube-system-diagnostics/actions/runs/35935022641),
  [both Macs](https://github.com/QubeTX/qube-system-diagnostics/actions/runs/35935022675),
  and [all three Linux targets](https://github.com/QubeTX/qube-system-diagnostics/actions/runs/35935022630).
- [Final qualification/publication](https://github.com/QubeTX/qube-system-diagnostics/actions/runs/35936902455)
  passed, including public immutable-v2 Cargo migration and managed Linux
  lifecycle checks. The synthetic-prior tests did **not** cover the real
  3.1.3 ABI-1-to-ABI-2 updater failure described below.

An independent local download verified all 59 assets, all 28 SHA256SUMS entries,
26 subject attestations, 26 sidecars, six stable installer/compatibility routes,
the six-target manifest and the crate index. The private local artifact record
is `target/v4-qualification/public-release/verification.json`; public subjects
remain available on the immutable release. No published tag or asset was replaced.

## Actual installed Windows copy

The existing managed PowerShell owner and installation path were preserved.
Running the checksum/attestation-verified public managed installer completed
the upgrade after the older automatic updater failed. The CLI reports 4.0.0;
the GUI self-test reports success, product 4.0.0, ABI 2 and engine schema 2.

| Installed component | SHA-256 matching the public archive |
|---|---|
| CLI | `e1c54bd86d89e2e574475b289fd6da74e302ab8d61a5fd0a70edeb8858459a18` |
| GUI | `f49f2614c2ae9ffdfac1bee65a3d2df9d475063f37d95776902a009b32e99978` |
| Engine | `f0cf5fd7a9853bab49c5036c807ba0d93ced7aa9a7dbb6aecd4fade1a1709b10` |

Every GUI archive file, including the manifest, icons and notices, matches its
installed counterpart. Settings were preserved byte-for-byte across installation.
The Start-menu shortcut targets the installed GUI and uses its embedded icon.
The actual installed app was launched and visually inspected: the v4.0.0 label,
custom title-bar icon, live overview, charts and navigation are present. This
is not a new resource soak or physical Windows tray acceptance.

## Defects found by the public-copy check

1. The immutable 3.1.3 updater hardcodes GUI ABI/schema 1. It rejects the v4
   ABI/schema 2 self-test and restores the older CLI. For this major upgrade,
   run the official installer matching the existing owner/format/edition,
   without uninstalling first. The actual same-owner managed recovery above
   is verified. Do not claim the original `sd300 update` path passed.
2. Public Windows 4.0.0 `update --json` reproducibly reports empty release JSON.
   Windows PowerShell and PowerShell 7 both exit successfully without executing
   under detached launch with null stdin. A direct eight-case comparison of
   detached/hidden-console and file/pipe output isolates the host-launch flags.
   [PR #11](https://github.com/QubeTX/qube-system-diagnostics/pull/11) gives only
   PowerShell hosts a hidden console while preserving bounded job ownership.
   Local real-host tests, 231 root library tests, eight CLI tests, three worker
   tests, 17 engine tests and lint pass. Hosted/corrective-publication evidence
   is tracked in task #p4h; this correction is absent from public 4.0.0.

The source fix must not be copied over the user's public installation and
called a released version. Publication of another version requires reconciling
the operator's single-release plan and explicitly expiring performance policy.

## Website and remaining evidence boundaries

[Website PR #16](https://github.com/QubeTX/qube-machine-report-homepage/pull/16)
merged as `954b574b1cc5aa20e8e7a906337cd33f34e13f97` and deployed successfully.
The [production SD-300 page](https://reports.qubetx.com/sd300) shows actual
desktop and TUI screenshots. Desktop/mobile browser checks verified image
decoding and dimensions, keyboard selection, pressed states, full-size links,
no horizontal overflow and no page errors.
[Website PR #17](https://github.com/QubeTX/qube-machine-report-homepage/pull/17)
adds the major-upgrade and Windows recovery instructions discovered above.
It merged as `a63b90c807f1b7b97b694b21fba01a17324c1bfb`; production deployment
6627424406 succeeded, and the same desktop/mobile checks passed again against
the actual production site, including the upgrade and workaround text.

The [original next-version targets](../../next-version-targets.md) and
[candidate qualification](README.md) retain all original and approved verdicts.
Hosted native/provider fixtures do not prove every physical device, screen
reader or tray surface. Existing physical-acceptance tasks remain open.
