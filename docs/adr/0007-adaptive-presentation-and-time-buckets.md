# 0007 — Adaptive presentation and captured time buckets

Status: Accepted for the authorized v4 implementation, 2026-09-23.

## Context

The old section renderers cloned and sorted process inventories inside rendering,
clipped compact layouts, had no row inspection and converted missing history values to
zero. Stable interaction and trustworthy graphs require a separate presentation step.

## Decision

Prepare complete inventories outside rendering. Retain process selection using PID and
creation time; if that process disappears, keep an explicit ended-selection state until
the user chooses another row. All nine sections support filtering, selection and inspection
in both modes. At compact widths, inspection occupies the table region; wide terminals
show aligned charts and a persistent contextual inspector. Esc first closes transient UI.
The old j/k, section, mode and process-sort keys remain; empty inventories clamp selection.

Pause copies presentation values, never collector handles. The session keeps draining
latest results into a separate snapshot; resume displays the newest capture. Historical
points not presented during the pause become gaps rather than an invented continuous
trace. TUI preferences live under `tui`; a GUI-only settings write preserves that namespace.
Mouse capture is opt-in and restored by an RAII guard. ASCII and no-color fallbacks are
applied to the bounded terminal buffer. No timer runs purely to animate numeric readings;
focus changes are immediate and the default honors reduced motion.

Both frontends retain timestamped finite capture buffers and project fixed time buckets.
Empty buckets stay absent, measured zero stays zero, and CPU/GPU temperatures never share
one series. TUI dots identify missing buckets explicitly. The pinned SDK line renderer
connects finite points across NaNs; GUI histories therefore use independent bars, whose
existing renderer omits non-finite buckets. This avoids a dependency patch and prevents
interpolation across missing data. Native tests supply an explicit clock and capture time.

## Evidence and limits

Deterministic tests cover all sections/modes at compact and wide dimensions, fallback
rendering, large inventories, PID reuse, missing rows, filters, pause and timestamp gaps.
A Windows PTY exercised startup, process filtering, pause/resume, inspection and clean
terminal exit. Native platform, performance and full interaction qualification remain
required for the final release; unit tests do not establish latency or hardware accuracy.

## macOS terminal readiness, 2026-09-23

The native Apple Silicon PTY repeatedly retained the old section after the first
key immediately following a resize, while collection, repaint and terminal decoding
continued. Structural failure artifacts identify this separately from provider hangs.
The default crossterm event source returns early from a batch of edge-triggered mio
events when it encounters SIGWINCH; another input readiness in that batch can be lost.

On macOS enable crossterm's documented `use-dev-tty` backend, which uses ordinary
file-descriptor polling and re-observes pending input. The upstream
[feature documentation](https://docs.rs/crossterm/0.29.0/crossterm/)
and [macOS kqueue discussion](https://github.com/crossterm-rs/crossterm/issues/500)
support this choice. Windows and Linux retain their current backends. No dependency
cache is modified. Native PTY qualification includes thirty immediate resize/key
pairs without settling delays or key retries; both Mac architectures are the oracle.
