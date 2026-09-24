# Post-release Windows CI HTTP 403

Date: 2026-09-24
Source: 430e6c1, [CI 36056348704](https://github.com/QubeTX/qube-system-diagnostics/actions/runs/36056348704)
Correction: 38d1edf, [PR #13](https://github.com/QubeTX/qube-system-diagnostics/pull/13)

## Observed failure

The Windows job's opt-in live release check fails after both powershell.exe and
pwsh.exe return HTTP 403 from api.github.com. Both processes execute and report
the HTTP failure; this is distinct from the old detached-process silent exit.
All six native GUI jobs, Linux/macOS root jobs and security checks pass in that
run. The dependent cargo-dist plan is skipped because the Windows job failed.

The failure log retains only status 403, not response headers or a server
reason. Anonymous-IP quota exhaustion is plausible but unproved. GitHub
[documents](https://docs.github.com/en/rest/using-the-rest-api/rate-limits-for-the-rest-api)
an IP-scoped anonymous quota and permits an authenticated Actions token for CI.
Do not relabel the historical failure as a confirmed rate limit.

## Correction boundary

Only the opt-in Rust test reads the step-scoped read-only Actions token. The
real PowerShell request resolves the token from its inherited environment; no
secret value is interpolated into arguments, fixtures, logs or files. The
production transport and installed updater remain unchanged and unauthenticated.

The test still contacts the actual public latest-release endpoint, executes the
owned bounded helper, captures output and validates real JSON. It does not use
SD300_CI_RELEASE_TAG or accept an HTTP failure as success. On rejection, retain
only numeric allowlisted quota/reset/retry headers so a future failure is
diagnosable without arbitrary HTTP bodies or request headers in logs.

## Evidence

- Local formatting and all-target clippy pass.
- All 27 updater unit tests pass; the separately invoked opt-in live test
  returns public v4.0.1 without authentication.
- The installed public Windows CLI independently reports version 4.0.1 and
  `sd300 update --json` returns success, already-current, through its recorded
  PowerShell installer channel. No installation mutation is needed.
- Hosted Windows live authentication, PowerShell 5.1/7 installer discovery,
  root tests and clippy pass in
  [CI 36059748041](https://github.com/QubeTX/qube-system-diagnostics/actions/runs/36059748041).
  Linux/macOS root jobs, security and release planning also pass. This receipt
  covers the changed CI request path; it is not a new product performance run.

PR #13 merged as f36eeeb after the entire Windows core job passed. The routine
six-target GUI rerun is still running at this receipt; no product code changed
and this record does not claim the whole workflow is green.

The operator's separate Intel refresh acceptance is in ADR 0020. It does not
waive this HTTP check. Published 4.0.1 assets and tag remain immutable.
