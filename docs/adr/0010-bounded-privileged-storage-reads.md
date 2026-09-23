# 0010 — Bounded privileged storage reads

Status: Accepted for the unpublished v4 candidate, 2026-09-23.

Ordinary monitoring remains unprivileged. An optional SMART installation is not
permission to elevate, and opening Storage is not permission to read a device
with administrator privileges. The TUI's selected-drive **A** action and the
GUI's **Review read…** action prepare a separate confirmation. The core validates
the current whole-device inventory selection, discovers and verifies smartctl,
and supplies the device, absolute helper path, SHA-256 and fixed operation to
both interfaces. Refusal preserves monitoring and the previous captured result.

Preparation runs in an owned CLI subprocess with a ten-second deadline. After
explicit confirmation, another owned, unelevated CLI broker invokes Windows
`runas`, macOS `osascript` administrator authorization, or Linux `pkexec` with
its internal text agent disabled. The broker isolates potentially blocking OS
authentication from the engine library. In particular, an unelevated parent
must not synchronously wait on a setuid child that it cannot terminate. Linux
without PolicyKit or an appropriate authentication agent keeps ordinary reads
and explains the missing authorization facility; it never captures a password
inside the raw terminal UI or installs a persistent elevation policy.

The elevated worker connects to a session-local IPv4 loopback listener with an
OS-random 256-bit nonce. The confirmed request and typed result are framed in
memory with a 64 KiB maximum; handshake reads have an absolute deadline. No
diagnostic result file or arbitrary shell command crosses the boundary. The
worker requires an administrator/root token, matching product version, an
allowlisted whole-device path and the prepared helper hash. It copies verified
helper bytes into a private temporary directory before execution, preventing a
later replacement of the original path from changing this invocation. Only
`smartctl --json --all --nocheck=standby,3 DEVICE` is allowed, with a twelve-second
command deadline. No self-test, repair, daemon or device-setting command exists
in this interface.

The overall authentication/action deadline is one minute. Closing the callback
signals cancellation to the elevated worker's owned child process. A worker-only
watchdog requests cancellation at twenty seconds and exits after a short cleanup
allowance; this watchdog never lives in the unloadable GUI engine. A late OS
approval after cancellation cannot obtain the now-closed confirmed request and
therefore cannot start a device read. Kernel tasks that cannot complete even a
pending termination remain an OS limitation, not something a userspace timeout
can certify away. The frontends do not wait on such native work.

Results retain their actual capture timestamp and provider observation. They
remain separate from periodic health data, so the next refresh cannot silently
replace or retimestamp an explicitly requested result. Both frontends consume
the same findings. Schema-2 exports include these results; redacted exports
remove structured device identifiers, serials, helper paths and provider detail.
Schema-1 output is unchanged.

Deterministic tests cover operation consent, device allowlists, helper changes,
framing limits, absolute handshake timeout, fault import, disconnect cancellation,
redaction and GUI device-selection stability. Hosted native qualification uses
preauthorized runner privileges and a compiled synthetic helper that never opens
a disk. This tests privileged worker behavior on the actual OS/architecture. It
does not prove interactive authorization-dialog acceptance or physical-device
SMART accuracy; those are separate acceptance evidence.

Primary references:

- [Windows ShellExecuteEx authorization and cancellation](https://learn.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-shellexecuteexw)
- [Apple command quoting](https://developer.apple.com/library/archive/documentation/LanguagesUtilities/Conceptual/MacAutomationScriptingGuide/CallCommandLineUtilities.html)
- [Apple administrator execution](https://developer.apple.com/library/archive/technotes/tn2065/_index.html)
- [PolicyKit authentication, environment and exit semantics](https://github.com/polkit-org/polkit/blob/master/docs/man/pkexec.xml)
