# ADR 0013: GUI monitoring hierarchy and complete process pages

Date: 2026-09-23
Status: Accepted for the unpublished v4 candidate

## Context

The operator expanded v4 to include a GUI design and interaction review. Eleven live Windows captures covered every monitoring section, Settings and bandwidth consent. Large headings and static identity displaced measurements; optional setup preceded monitoring; explanatory text clipped. Process values changed while the old thirty-sample ordering remained fixed. The compact eight-row presentation also hid half of a sixteen-row engine page before Next advanced to the following page.

## Decision

Retain Warm Carbon, the pinned Native SDK, independent GUI settings and the existing bounded rendering/sampling architecture. Put live readings, histories and central findings before static identity. Expose audience mode in the header. Wrap explanations and consent. Group network diagnosis and bandwidth consumption separately, and put storage setup with storage. Keep expensive actions explicit and preserve the separate M-Lab opt-in.

Present every row of each bounded sixteen-process page and preserve the engine's current rank on each capture. Use readable sort descriptions rather than unsupported font glyphs. Process identity still includes creation time. Select active interfaces that contribute to the aggregate before truncating the interface display; retain the full total count and a stable name/MAC-derived view identity.

Treat absent optional observation detail as absent instead of substituting pending text. Distinguish pending, delayed and interrupted readings; do not describe uncaptured zero defaults as low resource pressure. Technician metadata uses the relevant topic's sequence and capture time. Unified graphics memory remains distinct from dedicated video memory.

The operator subsequently requested Makira and Gail Rock as the main GUI fonts. Use the supplied static Makira face for heading/display text, Gail Rock for body/navigation/buttons, and Plex Mono for technical values. A reviewed downstream heading-font token resolves the same face in intrinsic sizing, wrapped text, selection geometry, overflow audit and painting. Do not switch only the rendered font after measuring another face. Embed the unchanged font files from private, hash-verified build inputs on every platform; preserve the pinned upstream SDK archive and put the downstream change in the reviewed patch.

## Verification and limits

The review record is `docs/qualification/v4/gui-ux-review.md`. Fixtures cover process ordering and page completeness, repeated capture warmup, available observations without detail, large virtual-interface inventories, startup assessments, consent paragraphs and control bounds across all sections and both modes. Real Windows interaction checks cover resizing, search, focus, mode switching and confirmation dismissal.

This changes GUI presentation intentionally and supersedes the prior eight-row preview and thirty-sample ordering. It does not waive resource budgets, reduce collector cadence or establish screen-reader support. The canvas accessibility limitation remains tracked by #acc. Native macOS/Linux qualification remains separate from Windows visual evidence.
