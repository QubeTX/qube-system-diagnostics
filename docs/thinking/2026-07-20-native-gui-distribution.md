# Native GUI distribution decision

Date: 2026-07-20
Status: accepted for the v3.0.0 qualification branch
Confidence: high
Revisit: after the six-target packaged feasibility gate, or immediately if Native SDK gains a signed reproducible on-device build service

## Decision

Ship SD-300's Native SDK GUI as centrally built, portable, target-specific release
artifacts inside the existing installers and updater routes. Do not make
`sd300 build app` an ordinary install, update, repair, or trust path.

Every distributable Zig build must name both its target and baseline CPU contract.
Host-native CPU defaults are allowed only for an explicitly labelled developer build.
The Rust engine is likewise built for the release target, and the GUI and engine are
qualified and attested as one composite product.

## Question and success criteria

The immediate question was whether compiling the GUI on each user's computer would be
a better response to a Native SDK test application's AMD-to-Intel illegal-instruction
failure. The actual need is a reliable GUI on all six SD-300 targets that preserves the
current CLI/TUI lifecycle and can be installed, repaired, rolled back, verified, and
uninstalled by its proven owner.

The top criteria are:

1. Cross-machine reliability and compatibility.
2. Platform trust, provenance, and reproducibility.
3. Preservation of the existing install/update/uninstall contract.

## Facts and assumptions

- The failing Windows release contained `EXTRQ`, an AMD SSE4a instruction, because its
  Zig build inherited the hosted runner's CPU features. An isolated explicit baseline
  Windows build ran on the affected Intel machine. This is a controlled difference in
  the build target, not evidence that Native SDK applications must be built by users.
- Native SDK forwards `-D...` arguments to Zig. Zig's standard target options otherwise
  use the build host.
- SD-300's GUI is not Zig-only. A local full build also needs Rust and the target's native
  linker/toolchain; Linux additionally needs development-time GTK dependencies.
- Different local builds produce different executable hashes. They cannot use SD-300's
  centralized macOS Developer ID signing/notarization or its normal Windows publisher
  identity, and they are not the exact artifacts covered by release attestations.
- A baseline prebuilt GUI has the same runtime efficiency target as a locally compiled
  GUI. Local compilation changes installation cost and code generation, not the Native
  SDK architecture or collection workload.
- Assumption to verify: explicit baseline target flags remain supported by every pinned
  Native SDK/Zig host lane. The six-target feasibility gate tests this before full UI work.

## Options considered

Scores are 1 (poor) through 5 (strong). Weights reflect the release contract rather
than developer convenience.

| Option | Reliability 30% | Trust/provenance 25% | Lifecycle fit 25% | Runtime efficiency 10% | Delivery 10% | Weighted |
|---|---:|---:|---:|---:|---:|---:|
| Target-specific prebuilt Native SDK GUI | 5 | 5 | 5 | 5 | 4 | **4.9** |
| End-user full Zig/Rust build | 2 | 1 | 1 | 5 | 1 | **1.8** |
| Prebuilt default plus optional local "forge" | 3 | 3 | 2 | 5 | 1 | **2.8** |
| Locally build only Zig UI around a prebuilt Rust engine | 3 | 2 | 2 | 5 | 2 | **2.7** |
| Replace the GUI with Electron | 4 | 5 | 4 | 2 | 3 | **4.0** |
| Publish source only and document manual builds | 1 | 2 | 1 | 5 | 4 | **2.0** |

The strongest runner-up is Electron: mature packaging and signing would reduce framework
risk. It loses because the approved Native SDK path has already passed dynamic-library
loading, avoids a bundled browser runtime, and better fits the strict resource budget.
If Native SDK fails the six-target package gate, Electron remains the explicit fallback;
end-user compilation does not.

## Build and verification contract

- Release wrappers must pass an allowlisted target and `-Dcpu=baseline`; a missing or
  unknown target is a hard failure.
- CI records the Zig version, target, CPU model, Native SDK pin, Rust target, and artifact
  hashes. No distributable command may rely on implicit host defaults.
- Windows qualification includes launching the packaged result on a CPU vendor/family
  different from at least one build host, beginning with the current Intel Alienware.
- The exact installed bytes pass Native SDK strict checks, GUI self-test, CLI verification,
  installer discovery, rollback, uninstall, checksum, SBOM, attestation, and path-leak checks.
- Same-version missing-GUI repair remains `sd300 update`; it downloads the centrally built
  companion. It never invokes a compiler on a customer's machine.
- Developer source-build instructions may exist, but they are non-owner, unsupported
  artifacts and cannot replace a signed/notarized release installation.

## Defeaters and reflection

This decision should be reopened if a required target cannot consume a portable Native SDK
artifact, if baseline code generation still emits target-specific instructions, or if a
platform introduces a trust policy that rejects the centrally produced package. A material
Native SDK feature that reproducibly builds, signs, and attests identical bytes remotely
could also change the evaluation.

The early warning is any artifact whose target/CPU record is missing, any cross-CPU launch
failure, or any mismatch between staged and published hashes. The first formal reflection is
the six-target feasibility-gate review; the second is the unpublished v3.0.0 draft review.

## Sources

- [Native SDK CLI](https://native-sdk.dev/cli)
- [Native SDK packaging](https://native-sdk.dev/packaging)
- [Native SDK package distribution](https://native-sdk.dev/packages)
- [Native SDK code signing](https://native-sdk.dev/packaging/signing)
- [Zig build system](https://ziglang.org/learn/build-system/)
- [Apple Developer ID](https://developer.apple.com/support/developer-id/)
- [Microsoft SmartScreen reputation](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/smartscreen-reputation)
- [GitHub artifact attestations](https://docs.github.com/en/actions/concepts/security/artifact-attestations)
