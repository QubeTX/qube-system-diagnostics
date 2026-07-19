TT;DR: Give every diagnostic value explicit provenance and an honest unavailable/invalid state so missing Windows or macOS telemetry cannot look healthy. This is the semantic foundation for parity work.

## Why
Derived from live Alienware exploration and the macOS capability research. Current collectors frequently use empty vectors, zeroes, or generic defaults where the source is unavailable.

## Plan
Add reusable observation, availability, validity, freshness, source, unit, scope, confidence, access-tier, and sensitivity types plus a capability registry. Migrate health decisions and UI rendering incrementally without cloning `SystemSnapshot`.

## Impact
Unknown and permission-gated data becomes explicit. UI and JSON contracts will change for v2, so fixtures and documentation must move together.

## Acceptance
Health rules cannot infer good state from missing data, and Technician mode can explain source and unavailability.

## Verification
- [x] Observation invariants have unit tests
- [x] Missing values never produce healthy status
- [x] Capability availability distinguishes unsupported, permission denied, unavailable, contradictory, error, and valid
- [x] Redaction metadata prevents identifiers from leaking by default

## Status
Done. Typed observation status, source/detail provenance, capability JSON, redacted snapshot JSON, and explicit unavailable UI states are implemented and tested.

## Activity
- 2026-07-18 14:45 - created from cross-platform accuracy analysis.
- 2026-07-18 16:31 - completed observation, capability, redaction, and unavailable-state implementation; privacy and rendering regression tests pass.
