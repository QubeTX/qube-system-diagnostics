TT;DR: Make a direct universal PKG the normal macOS installer and update artifact while retaining only the compatibility asset immutable old SD-300 clients actually require.

## Why
Direct operator decision to use PKG rather than DMG for the normal macOS experience, reconciled with TR-300/ND-300's legacy bridge discovery.

## Plan
Build a universal binary, sign with Developer ID Application, package with Developer ID Installer, notarize, staple, publish versionless PKG, and update through exact verified PKG bytes. Test shell-to-PKG and PKG-to-shell fresh takeover. Audit v1.4.x updater filenames to determine whether a legacy bridge is required.

## Impact
Normal Mac users get one direct installer. Release credentials require the existing encrypted P12 private keys and notary credentials; the provided `.cer` alone is insufficient.

## Acceptance
Intel and Apple Silicon native jobs prove signature, notarization, install, update, downgrade, takeover, receipt identity, and uninstall.

## Verification
- [ ] Direct PKG is signed, notarized, stapled, and Gatekeeper-valid
- [ ] Universal payload runs natively on Intel and Apple Silicon
- [ ] PKG-channel update opens or invokes the exact verified PKG
- [ ] Shell/PKG fresh takeover works in both directions or fails before mutation
- [ ] Immutable v1.4.x updater compatibility is proven

## Status
Implementation ready but externally blocked. The direct universal PKG build/sign/notarize/staple workflow and Intel/Apple Silicon lifecycle matrix are complete; the supplied `.cer` has no private key. The three signing identity/team variables were copied from TR-300, but #ga2 still requires the seven certificate/notary secrets already configured in TR-300 and ND-300.

## Activity
- 2026-07-18 14:45 - created from the direct-PKG decision.
- 2026-07-18 16:31 - implemented direct PKG, dual Developer ID signing, notarization, exact receipt/payload validation, same-channel update, takeover, and uninstall; copied the three public signing variables and confirmed the seven private certificate/notary secrets remain unavailable to this repository.
- 2026-07-18 17:01 - upgraded Intel/Apple Silicon validation from an already-current origin check to a real synthetic-v1.9.9 direct-PKG update into the signed candidate; execution still awaits Apple secrets.
