# SD-300 bundled fonts

SD-300 embeds the TrueType faces in `gui/src/fonts` directly in the Native SDK executable so
the interface is deterministic and does not depend on fonts installed on the
host:

- `Makira-Regular.ttf` — page headings, product identity and large diagnostic
  numerals. Copyright 2025 Yukita Creative.
  The publisher must retain proof that its Makira license permits desktop-app
  embedding and redistribution before a public release is approved. Because
  this repository is public, the commercial source font is Git-ignored and is
  reconstructed only on trusted build runners from two encrypted repository
  secrets. `scripts/prepare-gui-fonts.mjs` verifies its reviewed SHA-256
  before any build uses it; the source font is never uploaded as a repository
  or standalone release asset.
- `Gail-Rock-Regular.ttf` — body copy, navigation and controls, paired with
  Makira at the operator's request on 2026-09-23. The supplied static TrueType
  face is embedded unchanged. Its source is also Git-ignored and reconstructed
  from the private `SD300_GAIL_ROCK_FONT_BROTLI_BASE64` build input, checked
  against `toolchain-lock.json`; it is not a standalone distribution asset.
  The publisher must retain the corresponding commercial embedding rights.
- `IBMPlexMono-Regular.ttf` — secondary face for compact measurements,
  technical labels, versions, and provenance. IBM
  Plex is distributed under the SIL Open Font License 1.1; see
  `IBM-PLEX-LICENSE.txt`.

Do not replace, subset, or add weights without rerunning Native SDK strict
font registration, screenshot, layout, package-size, and redistribution
checks on every release target.
