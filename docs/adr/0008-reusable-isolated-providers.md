# 0008 — Reusable isolated providers

Status: Accepted, 2026-09-23. Extends ADR 0006 without moving native probes into UI threads.

## Decision

Each enabled non-fast lane owns at most one subprocess. The child accepts only a two-byte
sample/reset message for its fixed read-only topic. One request can be outstanding; no
input, output or reader-thread queue grows with time. Responses are atomically installed
in the parent's private temporary directory, size-checked and removed after consumption.
The parent owns the process group/job, deadline and cancellation; an error destroys the
worker before recovery. Child stderr/stdout remain bounded by the existing output monitor.

Session-local caches therefore survive between samples. Static graphics discovery is
keyed to the current DXGI device identities and expires after five minutes. Negative
optional-provider observations back off to five minutes, while successful telemetry is
always recollected. Explicit retry resets caches. Interface/disk identity changes trigger
discovery, and a long fast-sample gap resets rates and discovery after resume.

## Qualification

Fixtures prove negative caching, recovery and identity invalidation. A real collector
protocol test sends multiple requests to one PID, requests cache reset and verifies bounded
cancellation/shutdown. A hung-helper fixture exercises timeout without requiring native
return or pipe EOF. Hosted native runs validate each OS's process ownership and atomic-file
behavior. Release CPU/memory/handle and two-hour soak gates remain separate from these tests.
