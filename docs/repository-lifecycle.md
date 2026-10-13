# Repository Lifecycle

Status: Draft
Related RFCs: RFC-0015 (primary), RFC-0008, RFC-0016 (deferred)
Related docs: [repository-layout.md](repository-layout.md), [storage-design.md](storage-design.md), [cli-spec.md](cli-spec.md)
Owner crate: `functor-repository` ([ADR-0009](adr/ADR-0009-functor-repository-crate.md))

## Purpose

Defines the lifecycle of a Functor repository in the MVP: how it is created, discovered, used concurrently and how it evolves, with precise behavior for each transition.

## States

```text
(absent) --init--> Empty --create/update/add--> Populated --validate--> Verified (informational)
```

"Verified" is not stored state: it is the outcome of the latest `validate` run, visible only through the `validation.completed` event.

## Initialization

`functor init [PATH]` ([cli-spec.md](cli-spec.md)):

1. Resolve the target directory (create it if missing).
2. Refuse (exit 4) if the target already contains `.functor/`, or if an ancestor is a repository and `--nested` is not given.
3. Create `.functor/`, `artifacts/`, `relationships/`, `events/` and `.gitkeep` files.
4. Write `.functor/repository.toml` **last**, atomically. This file is the commit point: a directory without it is not a repository.
5. Report the repository id and path.

Failure atomicity: if any step fails, the directories created by this invocation are removed on a best-effort basis; because `repository.toml` is written last, a leftover partial structure is never recognized as a repository and a retry succeeds.

Init is **not idempotent** by design: running it twice is an error, which prevents silently overwriting a repository id.

## Discovery

Every command except `init` finds the repository by walking from the start directory (`--repo` or the current directory) up to the filesystem root, looking for `.functor/repository.toml`. The first match wins; nested repositories therefore shadow their parents. No match: exit 5.

Opening checks `format_version` before reading anything else ([storage-design.md](storage-design.md)). The one exception is `functor validate`, which uses the same walk but does not open the repository: an unsupported or malformed `repository.toml` is reported as a finding (`V0013`, `V0001`) rather than refused with exit 5.

## Normal Operation

Artifacts and relationships are created, updated (artifacts only), queried and validated. The evolution rules are:

- Identity never changes. Slugs and artifact fields may change; every update increments `revision`.
- Relationships are immutable, created once.
- There is no deletion in the MVP. Mistakes are corrected by `artifact update`; a wrongly created relationship can only be removed by deleting its file by hand, after which `validate` confirms integrity. This is a known limitation recorded in [mvp-scope.md](mvp-scope.md).
- Events accumulate in `events/current.log` as informational history.

## Concurrency

- Multiple readers and one writer at a time per repository. Writers serialize on `.functor/lock` ([storage-design.md](storage-design.md)); a second writer waits up to the lock timeout and then exits 7.
- `watch` and read-only commands never block writers.
- Optimistic concurrency for updates through `--expect-revision`.
- Concurrent access from different machines (shared network folders) is unsupported in the MVP.

## Version Control

The repository is designed to live in git: entity-per-file, canonical serialization, no machine-specific paths. Recommended ignores: `.functor/lock`, `events/current.log`. Merge conflicts in an entity file are resolved by hand, and `functor validate` checks the result (duplicate slugs from parallel branches show as `V0004`, identical files with different names as `V0017`).

## Evolution of the Format

`format_version` changes only through an ADR that defines a migration command. The MVP has version 1 only ([storage-design.md](storage-design.md)).

## Archival

Archiving a repository means copying or committing the directory; nothing else is required because all state is in plain files and the layout carries no absolute paths. Read-only archives work: read commands do not write (the lock file is only created by writers). A formal archive state (marking a repository read-only, exporting bundles) is deferred with import and export (RFC-0015).

## Future

Versioning and history (RFC-0016), federation and synchronization (RFC-0009), repository export and import, deletion with tombstones and governance policies (RFC-0018).
