# ADR-0012: Workflow, local execution provider and execution safety

## Status

Accepted

## Context

[RFC-0007](../../rfcs/RFC-0007-Validation-and-Execution.md) says executions consume artifacts and produce outputs, but leaves provider protocols and sandboxing undefined. Phase 1 needs one bounded local provider. The MVP baseline says the CLI never executes repository content ([nonfunctional-requirements.md](../quality/nonfunctional-requirements.md)); Phase 1 relaxes that only for an explicit, permitted invocation. `unsafe_code = "forbid"` applies to Functor code.

## Decision

1. **Workflow artifact.** An artifact is executable only if `type = "workflow"` and its content is `format = "toml"` with `schema = "functor.workflow/1"`. Any other artifact is refused with `InvalidInput`. Nothing runs on repository open, `validate`, `query` or `watch`.
2. **One provider: `local-process`.** The workflow names `provider = "local-process"`. An unknown provider is `Unsupported` (new `FunctorError` kind, see the contract). The provider runs one child process per run from structured `program` plus `args`. **No shell interpretation**: `cmd /c`, `powershell -Command`, `sh -c` style invocations are not special-cased and are not supported as a feature; a workflow can still name such a program, and the grant (below) is what authorises it.
3. **Explicit grant.** Every execution requires an `ExecutionGrant` supplied by the caller, naming the resolved absolute program path and the effective limits. Library callers construct it and the library never prompts; the CLI builds it from `--grant-exec`, prints the resolved program and limits, and, when stdin is a terminal, also requires an explicit `yes` (anything else is `PermissionDenied`). With stdin not a terminal the flag alone suffices. No grant means `PermissionDenied`, before any capture or write. A grant is per invocation and is never read from repository content, so untrusted artifacts cannot grant themselves permission.
4. **Program resolution.** `program` is a bare name (resolved on the caller's `PATH`) or a repository-relative path (confined like content paths). Absolute paths are refused in the workflow. The resolved path and the SHA-256 of the program file are recorded as provenance. A missing program is `start_failed`, recorded as terminal evidence.
5. **Sandbox directory, not a security sandbox.** The child runs with its working directory set to a fresh directory `.functor/runs/<run-id>/work/`. Captured inputs are copied there under `inputs/<name>`; declared outputs are read only from `outputs/<name>` in that directory. Declared names follow the tag grammar; output paths may not contain `..`, be absolute or traverse a symbolic link. This is a path convention and an input/output boundary. **It does not stop the child from reading or writing elsewhere, or from using the network.** Documentation and CLI output say so.
6. **Environment.** The child environment is cleared, then populated from a closed default allowlist (`SystemRoot`, `PATH`, `TEMP`, `TMP` on Windows; `PATH`, `TMPDIR` elsewhere), plus names listed in the workflow `env.allow` that the grant also lists, plus literal `env.set` values. Names containing `TOKEN`, `SECRET`, `PASSWORD`, `KEY`, `CREDENTIAL` (case-insensitive) can never be allowed. Provenance records variable **names** only, never values, except literal `env.set` values, which live in the workflow artifact already.
7. **Network.** Not enforced by the local provider (accepted at review). Provenance records `network = "not_enforced"`. A workflow declaring `network = "none"` is refused rather than silently unenforced.
8. **Limits** (defaults and hard maximums in the [contract](../design/execution-and-evidence.md#limits)): wall-clock timeout, per-stream captured output, number and size of inputs and outputs, argument count and size. Exceeding an output limit truncates and marks `truncated = true`; exceeding the timeout kills the process tree and ends the run `timed_out`.
9. **Process-tree cancellation.** On timeout or cancellation the whole process tree is killed. On Windows this uses a Job Object with kill-on-close. Because Functor code forbids `unsafe`, the Job Object is created by a vetted third-party crate; the crate is chosen at the start of P1.3 under the [dependency policy](../architecture/dependency-policy.md) checklist (license, advisories, footprint), and P1.3 does not start until that is recorded. Where no tree-kill mechanism is available (non-Windows, informational), the provider kills the direct child and states the limitation.
10. **Collaborators are injected.** The provider (`ExecutionProvider`, core trait) receives a prepared plan and returns a provider result. Capture, persistence, relationships, validation and events belong to the orchestrator ([ADR-0013](ADR-0013-run-lifecycle-and-evidence-persistence.md)). The provider never opens the repository.
11. **No auto-retry.** A run whose side effects may have happened is never re-executed automatically.

## Consequences

- Safety controls are small and testable (each limit has a boundary test), but the provider is honest about not being a sandbox.
- A new Windows-only dependency enters `functor-runtime` only, not the lower layers.
- Shell-style workflows work only because the user grants the shell binary; this is visible in the grant prompt.

## RFC Impact

Subsets RFC-0007 and RFC-0001 execution (C-0001-5, C-0007-4) for local execution. Defines a provider boundary RFC-0007 leaves open; sandboxing and capability advertisement stay deferred to Phase 3 and Phase 6. A future RFC revision of RFC-0007 may supersede this.
