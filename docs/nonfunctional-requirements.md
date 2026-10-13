# Non-Functional Requirements

Status: Draft
Related RFCs: RFC-0008 (local-first, portability), RFC-0007 (reproducibility), RFC-0013 (deferred)
Related docs: [storage-design.md](storage-design.md), [cli-spec.md](cli-spec.md), [testing-strategy.md](testing-strategy.md)

## Purpose

Quality attributes the CLI MVP must meet beyond functional behavior, stated so they can be tested.

## Determinism and Reproducibility

- The same repository state and the same command produce byte-identical output (text and JSON). All collections have documented orders ([domain-model.md](domain-model.md)); no `HashMap` iteration order reaches output.
- The same operation sequence with the same injected clock and id source, starting from the same files, produces byte-identical files. Files created by the tool use the canonical form; updates edit in place and preserve existing comments and formatting ([ADR-0011](adr/ADR-0011-toml-edit-preserving-updates.md)).
- Output contains no machine-specific data (absolute paths appear only where the command is about a path: `init` and error messages).

## Portability

- **Supported and blocking platform: Windows 10 or later (x86_64).** It is the only platform the project can currently build, test and maintain.
- **Linux and macOS are best-effort and non-blocking.** The code should stay portable (the rules below exist for that reason) and CI may run those targets, but a build or test failure there does not block merging or the MVP exit criteria, and is fixed only when a contributor volunteers to maintain that port. A platform is promoted to supported when someone owns it.
- Filenames, line endings and path separators: see [repository-layout.md](repository-layout.md) and [storage-design.md](storage-design.md). A repository created on one OS must validate cleanly on the others, including after a git checkout with different `autocrlf` settings.
- No required network access, no required environment configuration, no daemon.

## Performance Budgets

Measured with a release build on an SSD-backed developer machine; verified by a benchmark job that **reports** and fails only on gross regression (more than 3x the stored baseline).

| Scenario | Repository size | Budget |
|----------|-----------------|--------|
| `artifact show`, `relation add` (single write) | up to 10,000 artifacts | 300 ms |
| `query dependencies` cold start (strict load + traversal) | 1,000 artifacts, 2,000 relationships | 200 ms |
| same | 10,000 artifacts, 20,000 relationships | 2 s |
| `validate` | 10,000 artifacts, 20,000 relationships | 3 s |
| `watch` event latency (append to print) | any | 1 s (notification) and 6 s worst case (polling) |
| Peak memory for the 10,000 artifact case | | 200 MB |

These sizes are the MVP design target. Larger repositories need an on-disk index or a different store, both explicitly deferred (RFC-0008 allows them). Single-write operations scan the artifacts directory for slug uniqueness; if that exceeds the budget, an in-memory-per-process slug index is the first optimization and does not change any contract.

## Reliability

- No data loss on crash, kill or power failure beyond the entity being written ([storage-design.md](storage-design.md) crash table).
- Never leave a partially written entity file visible under its final name.
- A damaged repository must still be diagnosable: `validate` never aborts on malformed entity files, it reports them.
- All commands are safe to interrupt with Ctrl-C.

## Security Baseline (local safety, not RFC-0013)

RFC-0013 trust and signing are out of scope. The MVP still must be safe on untrusted repositories:

1. **Never execute** artifact content, hooks, scripts or paths from a repository.
2. **Path safety:** entity file names derive only from validated ids; content paths are relative, normalized and confined to the repository root (symlink-safe). The CLI never writes outside the repository root.
3. **Bounded parsing:** entity files larger than 2 MiB and inline content larger than 1 MiB are rejected as `V0001`; JSON event lines larger than 64 KiB are skipped with a warning; nesting depth is limited by the parser defaults.
4. **No secrets in diagnostics:** the CLI never logs file contents; the event log holds only ids, slugs and counts.
5. **No network access** in the MVP; `notify` uses only local filesystem facilities.
6. `unsafe_code = "forbid"` in all Functor crates; dependencies audited with `cargo deny` ([dependency-policy.md](dependency-policy.md)).
7. Terminal safety: strings from repository files are escaped for control characters before printing in `text` mode, so artifact titles cannot inject ANSI sequences into a terminal.

## Observability

- Diagnostics are the `error:` and `warning:` lines on stderr and, with `-v`, the cause chain of an error. There is no logging framework in the MVP (no `tracing`, `RUST_LOG` or `-vv`).
- No telemetry, no phone-home.

## Usability

- Every error is actionable: names the offending reference and suggests the next command ([error-handling.md](error-handling.md)).
- Output is plain text with no color or ANSI sequences, so `NO_COLOR` and terminal detection are not needed.
- `--help` for every command includes at least one example. Help text is part of golden tests.
- Output works in a pipeline: stdout is flushed per line in `watch`; closed pipes (`| head`) end the process quietly with exit 0.

## Maintainability

- Library crates: `#![forbid(unsafe_code)]`, documented public API (`missing_docs` warning), no `unwrap` or `expect` in non-test code except with a justification comment.
- Test coverage expectations are in [testing-strategy.md](testing-strategy.md).
- Dependency edges enforced ([crate-dependencies.md](crate-dependencies.md)).

## Compatibility Promises (MVP freeze, see [mvp-scope.md](mvp-scope.md) exit criterion 8)

Stable once the MVP is declared complete: the TOML formats, JSON output schemas, rule codes `V0001` and onward, exit codes, event envelope `v = 1`, id formats. Everything else (Rust APIs, text output, help text) may change in a minor release.

## Accessibility and Internationalization

English only in the MVP. Text output uses plain ASCII punctuation and does not depend on color or Unicode box drawing. Artifact content is UTF-8 and printed unchanged (after control-character escaping).
