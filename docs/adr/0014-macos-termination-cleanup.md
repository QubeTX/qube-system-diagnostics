# ADR 0014: join collectors before AppKit termination

Date: 2026-09-23
Status: Accepted; native Intel and Apple Silicon verification passed
Related: ADR 0008, task #v4a

## Evidence

Run 35887372666 on 7b08a2d records clean GUI root exit on Apple Silicon while three owned collector processes remain alive under parent PID 1: slow, diagnostics and activity. These are running/idle processes, not a transient zombie accounting result. The earlier Intel run failed the same remaining-helper gate.

The GUI currently stops its Rust engine in a deferred cleanup after the native event loop returns. The external quit endpoint calls AppKit `terminate:`. Apple's [termination contract](https://developer.apple.com/documentation/appkit/nsapplication/terminate(_:)) explicitly says that final cleanup in `main` will not run. The native SDK observes termination for its own shutdown event, but SD-300's separately owned engine is outside that hook.

## Decision

Register a synchronous `NSApplicationWillTerminateNotification` observer while the engine is alive. It calls an exported, idempotent GUI cleanup function that disconnects the active engine pointer, stops collection and joins the Rust monitor before AppKit exits. Keep the library loaded; normal deferred cleanup still owns destruction and unloading when the event loop returns normally. Remove the observer before that ordinary cleanup.

Do not replace AppKit's delegate, alter menu/OS termination semantics, or weaken the remaining-helper gate. Windows and Linux termination paths retain their existing behavior. Sudden process termination is outside Apple's notification contract and is not claimed as orderly shutdown.

## Verification

A native Objective-C fixture checks synchronous notification delivery, duplicate-install prevention, removal and reinstallation. The existing full-bundle GUI smoke must then prove that real AppKit quit leaves no owned collectors on both Mac architectures. Windows native tests/strict bindings check the shared Zig bridge. CPU profiles remain diagnostic evidence, not performance acceptance.

CI run 35890420111 on source 78cc519 passes all six native targets, including the notification fixture and full-bundle orderly-quit checks on both Intel and Apple Silicon. This closes the identified shutdown defect; longer resource windows remain a separate release gate.
