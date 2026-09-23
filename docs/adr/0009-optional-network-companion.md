# 0009 — Optional network companion and result privacy

Status: Accepted for the v4 candidate, 2026-09-23.

ND-300 is independently maintained and installed. SD-300 invokes the public
4.0.1 executable contract, with synthetic fixtures derived from that tag's
`src/diagnostics/mod.rs`, `src/cli.rs` and SpeedQX measurement structures. Unknown
versions are rejected until a compatibility fixture is reviewed. Unreleased
4.0.2 source is not linked, copied or required.

The terminal's N panel and the GUI Network section use the same session-owned
controller. Standard and deep requests have fixed argument lists (`--json
--fast`, `--json --tech --fast`). They never request repair, DNS changes or
periodic bandwidth tests. One worker is active per session, with one completion
slot, bounded output and owned process cleanup. Monitoring and input continue.

SpeedQX is a distinct, confirmed action: Quick allows 90 seconds / 5 GB synthetic
payload and Deep allows 300 seconds / 20 GB. M-Lab consent starts false for every
confirmation and is passed only after explicit selection. The UI retains the
methodology's sustained application-throughput interpretation, nullable
directions, HTTP latency and PDV jitter. Protocol overhead and in-flight payload
overshoot remain limitations. The enclosing process gets 15 seconds of grace to
serialize results after its own deadline; that does not increase its test budget.

Exit statuses 0, 1 and 2 are diagnostic outcomes when supported JSON exists.
Imported `timed_out` rows mean incomplete observations. Interruption, malformed
output, unsupported versions, execution timeout, permissions and absence remain
distinct. Valid JSON already returned is retained even if execution is later
cancelled or times out. The public interface does not stream incremental JSON;
an externally terminated process may return no new usable result. Previous
completed results remain visible with their original capture time.

Capture uses bounded pipes in memory, without reader threads or waits for EOF
from inherited handles. POSIX FIONREAD and Windows PeekNamedPipe inspect
available bytes; a pipe has exactly one reader and no concurrent operation on
its handle. Reads, per-cycle draining and total output are bounded. The existing
worker-process protocol remains isolated from this result-capture path.

Raw imported evidence is session-only. The GUI and TUI expose the same bounded
technical detail projection; full imported evidence can appear only in an
explicit sensitive export. Redacted reports discard imported detail lines and
raw objects entirely and replace summaries with SD-300-owned state descriptions.
They do not attempt to sanitize arbitrary ND-300 strings with address regexes.

Setup is a separate consent flow. It must detect existing owners first, use the
official immutable distribution with verified hashes, verify the installed
executable, and leave ND-300 installed when SD-300 is removed. This ADR does not
declare setup or release qualification complete.

Sources:

- [Public ND-300 4.0.1 source](https://github.com/QubeTX/qube-network-diagnostics/tree/v4.0.1)
- [Public artifacts](https://github.com/QubeTX/qube-network-diagnostics/releases/tag/v4.0.1)
- [Microsoft PeekNamedPipe](https://learn.microsoft.com/en-us/windows/win32/api/namedpipeapi/nf-namedpipeapi-peeknamedpipe)

Verification covers compatible warning/failure exits, partial results,
interruption, malformed/unsupported output, explicit action arguments, consent
reset, redaction, in-memory output limits, inherited-pipe exit, TUI confirmation,
GUI bindings and the engine request boundary. Live bandwidth measurements are
not triggered by those tests.
