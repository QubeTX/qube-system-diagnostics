# ADR 0003 — Managed uninstall removes the receipt parent only when empty

Date: 2026-07-22
Status: Accepted
Related: `src/update.rs` (`windows_managed_cleanup_commands`), `scripts/validate-windows-self-update.ps1` (`Assert-ManagedUninstall`), `.tasks/tasks/cpl.md`, hosted run `29892216141`

## Context

Hosted Windows qualification (run `29892216141`, step "Exercise real same-channel
version transitions", channel `powershell-installer`) failed managed uninstall:
after deleting the owned receipt (`%LOCALAPPDATA%\sd300\sd300-receipt.json`),
cleanup ran `Remove-Item -LiteralPath '<receipt parent>' -Force -ErrorAction
SilentlyContinue`. When the parent contained unrelated files, Windows PowerShell
required a recursion confirmation prompt; the updater deliberately runs
`-NonInteractive`, so the prompt became a terminating failure. SD-300 rolled
back correctly ("the installed executable was restored", exit 2). `-Force` and
`-ErrorAction SilentlyContinue` do not suppress that prompt, and `-Recurse` is
unacceptable: the receipt file is owned, but its parent directory may contain
unrelated user or tool state.

## Decision

Replace the parent removal with an empty-only, nonrecursive delete executed by
the same Windows PowerShell 5.1 cleanup command string:

```
try{[IO.Directory]::Delete('<parent>',$false)}
catch [IO.DirectoryNotFoundException]{}
catch [IO.IOException]{if(($_.Exception.HResult -band 0xFFFF) -ne 145){throw}};
```

- Missing directory: ignored.
- Nonempty directory: Win32 `ERROR_DIR_NOT_EMPTY` (145, low word of HRESULT
  `0x80070091`) is the expected preservation outcome — ignored, directory and
  its unrelated contents preserved.
- Every other failure (access denied, locks, bad path): rethrown → PowerShell
  exits nonzero → `run_status` errors → `WindowsUninstallImageHandoff::finish`
  rolls the live image back. The fail-closed transaction contract is unchanged.

Hosted validation now proves ownership safety end-to-end: the fixture writes an
unrelated sibling file beside the receipt, runs the real `sd300 uninstall
--json`, requires byte-exact (Base64-compared) preservation of the sibling,
asserts no unexpected receipt-root entries remain, and removes the then-empty
root itself.

## Why this is sound

- The command string executes under System32 Windows PowerShell 5.1 (spawned by
  the compiled binary, `-NoLogo -NoProfile -NonInteractive`); every construct
  used (typed catch clauses, `HResult`, `-band`, `[IO.Directory]::Delete`)
  behaves identically on .NET Framework and PowerShell 7. CI's `pwsh`
  orchestration invokes the real binary, so the snippet is transitively proven
  on the authoritative 5.1 runtime.
- The semantics exactly mirror the already-reviewed native Rust cleanup used by
  the non-detached path: `remove_empty_managed_receipt_directory()` ignores
  NotFound/DirectoryNotEmpty and fails on everything else.
- Independent research (PowerShell/​.NET/Win32/dotnet-runtime primary sources,
  Terra x-high review) confirmed both the prompt root cause and the empty-only
  `Directory.Delete(path, false)` model.

## Rejected / deferred alternatives

- **`Remove-Item -Recurse`** — rejected permanently: turns a CI fix into an
  ownership violation against unproven directory contents.
- **Full-HRESULT/facility guard instead of low-word 145** — deferred to backlog:
  a coincidental low-word match would merely preserve a directory (conservative
  false-tolerate), and scope discipline in a release drive outweighs the
  marginal strictness.
- **`File.Delete` for receipt removal, forced child working directory outside
  the receipt tree** — deferred to backlog as hardening; neither is part of the
  failing path.

## Addendum (2026-07-22): the Windows PowerShell 5.1 `-Command` exit-code contract

The first hosted qualification of this fix still failed: the child
powershell.exe exited 1 even though the sibling was preserved correctly. Local
reproduction on real Windows PowerShell 5.1 proved the mechanism:
`powershell.exe -Command "<string>"` mirrors the FINAL statement's `$?` in its
exit code. A caught-and-swallowed terminating error (our tolerated
nonempty-parent catch) — and equally a failed `-ErrorAction SilentlyContinue`
cmdlet — leaves `$?` false, so a tolerated outcome in last position reports a
false failure. PowerShell 7 does not behave this way, which is why the earlier
PS7-only local proof missed it (and why hosted 5.1 remains the authoritative
gate).

Correction: the composed cleanup string now ends with a terminal `exit 0`.
Empirically verified on 5.1: tolerated outcomes exit 0 with the sibling
byte-intact; an uncaught `throw` aborts execution before reaching the marker
and still exits 1 (fail-closed preserved); the empty-parent case still removes
the directory and exits 0. This also removes the latent same-shape hazard for
whichever suppressed removal happens to be the string's last statement.

Lesson recorded for future agents: when validating updater PowerShell behavior,
the exception semantics AND the host exit-code contract are separate claims, and
each must be proven on Windows PowerShell 5.1 specifically.

## Consequences

- Managed uninstall succeeds while preserving unrelated receipt-root siblings;
  an empty receipt root is still cleaned up completely.
- A fresh exact-head hosted Windows Native Installers run is the authoritative
  proof gate; run `29892216141` remains failure evidence only.
- The deferred hardening trio is tracked on the task board backlog.
