# Non-Functional Requirements

Status: Accepted
Related RFCs: RFC-0008 (local-first, portability), RFC-0007 (reproducibility), RFC-0013 (deferred)
Related docs: [storage-design.md](../design/storage-design.md), [cli-spec.md](../cli/cli-spec.md), [testing-strategy.md](testing-strategy.md)

## Purpose

Quality attributes the CLI MVP must meet beyond functional behavior, stated so they can be tested.

## Determinism and Reproducibility

- The same repository state and the same command produce byte-identical output (text and JSON). All collections have documented orders ([domain-model.md](../design/domain-model.md)); no `HashMap` iteration order reaches output.
- The same operation sequence with the same injected clock and id source, starting from the same files, produces byte-identical files. Files created by the tool use the canonical form; updates edit in place and preserve existing comments and formatting ([ADR-0011](../adr/ADR-0011-toml-edit-preserving-updates.md)).
- Ordinary repository output contains no machine-specific data (absolute paths appear only where the command is about a path: `init` and error messages). Execution provenance intentionally records the resolved program path, platform and provider version; run ids/timestamps describe real invocations and are not a deterministic replay promise.

## Portability

- **Supported and blocking platform: Windows 10 or later (x86_64).** It is the only platform the project can currently build, test and maintain.
- **Linux and macOS are best-effort and non-blocking.** The code should stay portable (the rules below exist for that reason) and CI may run those targets, but a build or test failure there does not block merging or the MVP exit criteria, and is fixed only when a contributor volunteers to maintain that port. A platform is promoted to supported when someone owns it.
- Filenames, line endings and path separators: see [repository-layout.md](../design/repository-layout.md) and [storage-design.md](../design/storage-design.md). A repository created on one OS must validate cleanly on the others, including after a git checkout with different `autocrlf` settings.
- No required network access, no required environment configuration, no daemon.

## Performance Budgets

Measured with a release build on an SSD-backed developer machine by `scripts/bench.ps1` (a synthetic repository, the median of five runs after a warm-up). The benchmark job **reports** and fails only on gross regression (more than 3x the stored baseline in `docs/quality/benchmarks/baseline.json`, recorded on the development machine); in CI it is informational. Run `pwsh scripts/bench.ps1 -Artifacts 10000 -Baseline docs/quality/benchmarks/baseline.json -Enforce` to fail on any budget, and add `-UpdateBaseline` to re-record the baseline after a deliberate change. Measured at the end of the MVP: 90 ms (`artifact show`), 95 ms (`query dependencies`), 93 ms (`validate`) and 130-155 ms (`relation add`) at 1,000 artifacts; 565 ms, 590 ms, 560 ms and 1.1 s at 10,000, with a peak of about 41 MB.

| Scenario | Repository size | Budget |
|----------|-----------------|--------|
| `artifact show`, `relation add` (single write) | 1,000 artifacts, 2,000 relationships | 300 ms |
| `artifact show` | 10,000 artifacts, 20,000 relationships | 750 ms |
| `relation add` (single write) | 10,000 artifacts, 20,000 relationships | 1.5 s |
| `query dependencies` cold start (strict load + traversal) | 1,000 artifacts, 2,000 relationships | 200 ms |
| same | 10,000 artifacts, 20,000 relationships | 2 s |
| `validate` | 10,000 artifacts, 20,000 relationships | 3 s |
| `watch` event latency (append to print) | any | 1 s (notification) and 6 s worst case (polling) |
| Peak memory for the 10,000 artifact case | | 200 MB |

These sizes are the MVP design target. Every read loads and checks the whole repository (a damaged repository is refused, RFC-0008), so a command costs one full load at 10,000 artifacts (about 0.55 s measured) and `relation add` costs two (one to resolve the endpoints, one under the write lock); the budgets above include that. The loader reads and parses files in parallel, and the CLI reads the repository once per command. Larger repositories, or budgets at 10,000 artifacts close to the 1,000 artifact figures, need partial loading or an on-disk index, both explicitly deferred (RFC-0008 allows them); neither changes a contract.

### Phase 1 execution measurements

`scripts/bench-execution.ps1` builds a release CLI plus a test-only deterministic fixture. It reports direct-child and end-to-end medians separately (five samples after a warm-up); their difference estimates CLI/provider/capture/publication overhead, not child workload time. No new latency gate is introduced. The [recorded Phase 1 measurement](benchmarks/phase-1.json) includes the OS, architecture, toolchain, sampling policy, sampled no-op/echo CLI peak working set, unique captured-payload storage and replay/deduplication checks.

It also verifies the default 1 MiB per-stream cap with one extra byte, rejects a 16 MiB + 1 byte declared output without publication, and interrupts a 60-second child with a 1-second timeout. Exact/below/over input/output bounds and lowered stream bounds are exercised by the core/provider tests. Limits bound captured/published data and supervised runtime, **not** a child's overall memory, CPU or disk use: the provider is not a resource/security sandbox.

## Reliability

- No data loss on crash, kill or power failure beyond the entity being written ([storage-design.md](../design/storage-design.md) crash table).
- Never leave a partially written entity file visible under its final name.
- A damaged repository must still be diagnosable: `validate` never aborts on malformed entity files, it reports them.
- All commands are safe to interrupt with Ctrl-C.

## Security Baseline (local safety, not RFC-0013)

RFC-0013 trust and signing are out of scope. The MVP still must be safe on untrusted repositories:

1. **Never auto-execute** artifact content, hooks, scripts or paths on repository open, validation, query or watch. Phase 1 permits only explicit workflow execution with a caller-supplied grant and the [ADR-0012 controls](../adr/ADR-0012-execution-provider-and-safety.md); interactive CLI execution also requires confirmation.
2. **Path safety:** entity file names derive only from validated ids; content paths are relative, normalized and confined to the repository root (symlink-safe). Functor's store writes stay inside the repository root; a granted child has the caller's privileges and is not confined by the staging-directory convention.
3. **Bounded parsing:** entity files larger than 2 MiB and inline content larger than 1 MiB are rejected as `V0001`; JSON event lines larger than 64 KiB are skipped with a warning; nesting depth is limited by the parser defaults.
4. **No secrets in diagnostics:** the CLI never logs file contents; the event log holds only ids, slugs and counts.
5. **No network adapter** in the shipped Functor dependency tree; `notify` uses only local filesystem facilities. A granted child can use the network: provenance says `network = "not_enforced"`, and `network = "none"` is refused rather than silently ignored.
6. `unsafe_code = "forbid"` in all Functor crates; dependencies audited with `cargo deny` ([dependency-policy.md](../architecture/dependency-policy.md)).
7. Terminal safety: strings from repository files are escaped for control characters before printing in `text` mode, so artifact titles cannot inject ANSI sequences into a terminal.

### Evidence

`scripts/check-conformance.ps1` verifies that every test named here exists.

| Item | Evidence |
|------|----------|
| 1 | `cli:run::no_grant_prints_a_plan_and_writes_no_evidence_even_when_quiet`, `cli:run::invalid_bindings_provider_schema_and_timeouts_are_refused_before_a_run`, `rt:provider::the_resolved_program_must_be_the_granted_one`, `cli:run::terminal_permission_requires_an_explicit_yes_and_always_shows_the_plan` |
| 2 | `model:content_path_rejects_unsafe_paths`, `store:content_paths_are_checked_for_safety_and_existence`, `cli:artifact::content_paths_must_exist_be_files_and_stay_inside_the_repository` |
| 3 | `store:oversized_and_non_utf8_files_are_malformed`, `store:content_problems_are_classified`, `events:follow::a_line_over_the_limit_is_skipped_with_a_warning_and_the_neighbours_are_delivered` |
| 4 | `cli:events::the_event_log_after_a_typical_session_is_stable` (the log holds ids, slugs and counts) |
| 5 | `script:check-deps.ps1` (no network-capable crate in the shipped dependency tree) |
| 6 | `unsafe_code = "forbid"` in the workspace lints; `cargo deny check` in CI |
| 7 | `cli:artifact::show_neutralizes_control_characters_from_repository_data` |

Windows owner-death cleanup and recovery are pinned by `cli:run::killing_the_cli_releases_the_lease_kills_descendants_and_recovers_without_rerunning`. Timeout and explicit cancellation kill the entire job/process group; abrupt owner death is guaranteed by kill-on-close on Windows only. The provider does not claim abrupt-death cleanup on Unix.

## Observability

- Diagnostics are the `error:` and `warning:` lines on stderr and, with `-v`, the cause chain of an error. There is no logging framework in the MVP (no `tracing`, `RUST_LOG` or `-vv`).
- No telemetry, no phone-home.

## Usability

- Every error is actionable: names the offending reference and suggests the next command ([error-handling.md](../architecture/error-handling.md)).
- Output is plain text with no color or ANSI sequences, so `NO_COLOR` and terminal detection are not needed.
- `--help` for every command includes at least one example. Help text is part of golden tests.
- Output works in a pipeline: stdout is flushed per line in `watch`; closed pipes (`| head`) end the process quietly with exit 0.

## Maintainability

- Library crates: `#![forbid(unsafe_code)]`, documented public API (`missing_docs` warning), no `unwrap` or `expect` in non-test code except with a justification comment.
- Test coverage expectations are in [testing-strategy.md](testing-strategy.md).
- Dependency edges enforced ([crate-dependencies.md](../architecture/crate-dependencies.md)).

## Compatibility Promises (MVP freeze, see [mvp-scope.md](../planning/mvp-scope.md) exit criterion 8)

Stable once the MVP is declared complete: the TOML formats, JSON output schemas, rule codes `V0001` and onward, exit codes, event envelope `v = 1`, id formats. Everything else (Rust APIs, text output, help text) may change in a minor release.

## Accessibility and Internationalization

English only in the MVP. Text output uses plain ASCII punctuation and does not depend on color or Unicode box drawing. Artifact content is UTF-8 and printed unchanged (after control-character escaping).
