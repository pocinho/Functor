# Storage Design

Status: Draft
Related RFCs: RFC-0008 (primary), RFC-0015, RFC-0005
Related ADRs: ADR-0005, ADR-0006, ADR-0009, ADR-0010, ADR-0011
Owner crates: `functor-store` (entities), `functor-repository` (layout, lifecycle, lock, format version)
Related docs: [repository-layout.md](repository-layout.md), [artifact-format.md](artifact-format.md), [traits-and-interfaces.md](traits-and-interfaces.md)

## Purpose

Implementation guidance for the MVP storage layer: how `FileStore` achieves durability, consistency and determinism on a local filesystem, and how `MemoryStore` relates to it. The RFC-0008 principles are storage-agnostic semantics, durable identity, local-first and version preservation; this document covers the first two and the third, and records what is deferred for the fourth.

## Components

| Component | Crate | Responsibility |
|-----------|-------|----------------|
| Repository lifecycle: layout paths, `repository.toml`, `format_version` check, `init`, `open`, `discover`, the write lock | `functor-repository` | Implements `RepositoryManager` ([ADR-0009](adr/ADR-0009-functor-repository-crate.md)) |
| `FileStore` | `functor-store` | Implements `RepositoryReader`, `ArtifactRepository`, `RelationshipRepository` over an opened repository |
| `MemoryStore` | `functor-store` | Same traits, in memory. Used by unit tests, examples and as the second implementation in the conformance suite. It does not implement `RepositoryManager` |
| File codecs: `artifact_file`, `relationship_file` (store) and `repository_file`, `toml_support` (repository) | `functor-store`, `functor-repository` | Hand-written mapping between `toml_edit` documents and model types: `parse_*` (classifying every problem), `render_*` (canonical form) and `edit_artifact_file` (in-place edits). ADR-0005 and ADR-0011 |
| `fs` helpers | `functor-repository` | Atomic write, safe path joins, locking, shared with `functor-store` and `functor-events` |

Index structures are in-memory only and rebuilt on every snapshot (RFC-0008: indexes are implementation details and must not affect semantics). There is no on-disk index or cache in the MVP.

## Write Protocol

Every mutating operation follows the same sequence while holding the repository write lock:

1. Acquire the exclusive lock on `.functor/lock` (advisory file lock, see below). Waiting is bounded (default 5 seconds, configurable by `FUNCTOR_LOCK_TIMEOUT`); on timeout return `Locked`.
2. **Re-read** whatever the operation depends on (for example, scan slugs for a uniqueness check) so concurrent writers cannot interleave.
3. Validate the operation against current state (slug unique, endpoints exist, revision matches).
4. Produce the new file text in memory: **new** files are rendered in the canonical form; **updates** edit the existing document in place with `toml_edit`, changing only the affected values and `revision`/`updated` so comments, order, formatting and unknown keys are preserved ([ADR-0011](adr/ADR-0011-toml-edit-preserving-updates.md)). If the resulting text is identical to the current file, stop: nothing is written.
5. Write to a temporary file in the **same directory** (`<name>.tmp-<random>`), flush and `fsync` the file.
6. Rename the temporary file over the target (atomic replace; `rename` on Unix, `MoveFileExW` with replace semantics on Windows through `std::fs::rename`). For new files use a create-new rename so an existing id is never silently overwritten.
7. Best-effort `fsync` of the parent directory on platforms that support it.
8. Release the lock.
9. The caller publishes the event **after** the lock is released and the state is committed (ADR-0003).

Temporary files left by a crash have the `.tmp-` marker and are ignored by the loader (and reported as warning `V0016` by validation). They are never read as entities.

A single operation modifies **one** entity file, so it is atomic. Operations that must change two entities do not exist in the MVP.

## Locking

- Advisory exclusive lock on `.functor/lock` for writers; readers do not lock. Readers always see complete files because of atomic rename, but a snapshot taken while a writer works may miss an in-flight entity. This is acceptable: a snapshot is a consistent view of **committed** entities (each file is old or new, never partial).
- Implementation: the standard library's `File::try_lock` on `.functor/lock`, polled until the timeout (see [dependency-policy.md](dependency-policy.md)). It is exposed as `WriteLock::run(closure)` by `functor-repository`, because a guard cannot outlive the file handle it borrows; `functor-store` and `functor-events` both use it. The lock file is created on demand and is not part of repository state.
- Locks are per repository, not global. `watch` never takes the write lock.
- Cancellation: dropping a write future before the rename leaves only a temp file. After the rename the write is committed. No other state exists.

## Read Protocol

`snapshot()` and `load()`:

1. `functor-repository` opens the repository: read and parse `.functor/repository.toml` and check `format_version` first. `FileStore` receives the opened repository handle.
2. List `artifacts/` and `relationships/`, ignoring `.gitkeep` and files in the reserved set. Sort by file name for determinism.
3. Parse each file with the file codecs, then into a model value through the validating constructors.
4. Build indexes (id, slug, outgoing and incoming adjacency) in memory.

`snapshot()` stops with `Inconsistent` at the first problem. `load()` collects every problem as a `LoadIssue` and continues:

| LoadIssueKind | Validation rule |
|---------------|-----------------|
| `MalformedFile` (invalid TOML, missing or invalid field, file over the size cap) | `V0001` |
| `MissingIdentifier` | `V0002` |
| `DuplicateArtifactId` | `V0003` |
| `DuplicateSlug` | `V0004` |
| `InvalidIdentifier` | `V0005` |
| `SelfReference` (an acyclic relationship pointing at its own source; the model constructor rejects it, so the loader reports it instead of building it) | `V0009` |
| `DuplicateRelationshipId` | `V0008` |
| `UnsafeContentPath`, `MissingContentFile` | `V0010` |
| `UnsupportedFormat` | `V0013` |
| `MissingStructure` | `V0014` |
| `UnexpectedFile` (non-fatal for a strict load) | `V0016` |
| `FilenameMismatch` | `V0017` |
| `UnknownField` (non-fatal for a strict load) | `V0018` |
| `Unreadable` | `V0019` (info) and an `Unknown` outcome |

The enum and `SnapshotLoad::into_snapshot` (which turns fatal issues into `Inconsistent`) live in `functor-core`.

When two files claim the same id, the loader keeps the one with the lexicographically smaller file name and reports the other, so results stay deterministic.

## Determinism

- Directory enumeration order is never trusted: always sort.
- Files **created** by the tool follow the canonical form in [artifact-format.md](artifact-format.md); a property test asserts that `serialize(parse(serialize(x))) == serialize(x)` for new files.
- **Updates** preserve the existing document ([ADR-0011](adr/ADR-0011-toml-edit-preserving-updates.md)). The same operation sequence on the same starting files gives the same bytes. An update that changes nothing writes nothing.
- Timestamps come from the injected `Clock`; ids from `IdGenerator`.

## Path Safety

- Entity file paths are built **only** from validated ids, never from user text: `artifacts/` + `ArtifactId::to_string()` + `.toml`.
- Content paths (`ContentPath`) are joined to the repository root through a helper that rejects absolute paths, `..`, and any component that is a symlink resolving outside the root, then verifies the canonicalized result still starts with the canonicalized root.
- The CLI never follows `--repo` outside the user's explicit request, and never writes outside the repository root.

## Encoding and Platform Rules

- UTF-8 without BOM. Invalid UTF-8 is a load issue.
- LF in files we write; CRLF on read is tolerated (`toml_edit` parses it) so Windows checkouts with `autocrlf` still load. An in-place update keeps the line ending style of the existing file.
- No reliance on file modification times, case-sensitive paths or symlinks.
- Windows: long paths are not specially handled; ids keep names short (about 40 characters per file). Replacement of a file held open by another process (antivirus, indexer) may fail transiently: retry the rename with a short bounded backoff (up to 5 attempts, 20 ms apart) before returning `Storage`.

## Format Versioning and Migration

- `format_version` lives in `repository.toml` (see [repository-layout.md](repository-layout.md)) and is checked by `functor-repository` when opening.
- The binary supports exactly one version range per release (`min..=max`). Opening a repository above `max` returns `UnsupportedFormat`; below `min` returns `UnsupportedFormat` with a hint to run a migration.
- The MVP has only version 1 and **no migration command**. Introducing version 2 requires an ADR that defines `functor migrate`, its atomicity and its rollback story.
- Forward compatibility inside a version: unknown fields are preserved on rewrite because updates edit the document in place (see [artifact-format.md](artifact-format.md)).

## Crash Consistency Summary

| Failure | Result |
|---------|--------|
| Crash before rename | Old state intact; stray `.tmp-` file ignored |
| Crash after rename, before event append | State committed; event missing (events are informational; `watch` consumers may miss it) |
| Crash during event append | Possibly a truncated last line; the event reader skips a final line without a terminating newline and reports a warning |
| Crash during event log rotation (after the rename, before the first write) | `current.log` is missing; the next append derives `seq` from the newest archive name and creates `current.log` |
| Power loss after rename | File is either old or new on journaling filesystems; the parent-directory fsync narrows the window |

## Event Log Writes

Owned by `functor-events`, which depends on `functor-repository` for the log paths and the write lock. The event log uses the same lock as state writes for sequence assignment, size-based rotation and pruning (10 MiB, 10 archives, [ADR-0008](adr/ADR-0008-event-log-rotation.md)) and an append-only write of exactly one line per event. Details in [event-model-mvp.md](event-model-mvp.md).

## Deferred

Version history and snapshots (RFC-0016), pluggable backends (SQLite, git objects, remote), on-disk indexes, repository export and import, packaging, content-addressable storage, and migration tooling.

## Implementation Notes (M3)

Decisions taken while implementing `functor-repository` and `functor-store` that refine the sections above:

- **Blocking work.** The stores implement their logic synchronously (plain `std::fs`) and the async trait methods run it through `functor_repository::run_blocking` (`tokio::task::spawn_blocking`). `FileRepositoryManager` also offers `init_blocking`, `open_blocking` and `discover_blocking` for synchronous callers such as test factories.
- **Reads.** Every read (`get`, `resolve`, `list`, `snapshot`) is a strict load of the whole repository, O(N) in files. There is no per-entity fast path and no on-disk index in the MVP; M9 added the parallel loader and the per-process `CachingStore` (see the M9 notes below).
- **Writes.** Every mutation takes the write lock, strict-loads, runs the rules in `functor-store`'s `rules` module (shared with `MemoryStore`), then writes one file. A damaged repository refuses writes with `Inconsistent`. A no-op update returns before writing; the file is not touched (verified by modification time).
- **Loader semantics.**
  - Candidates are `*.toml` files sorted by name. `.gitkeep` is skipped; files with the `.tmp-` marker, other extensions and subdirectories are `UnexpectedFile`.
  - Duplicates: the first file in name order wins and the later one is reported (`DuplicateArtifactId`, `DuplicateSlug`, `DuplicateRelationshipId`) and **excluded**, so the lenient result stays internally consistent.
  - `FilenameMismatch` keeps the entity (its id inside the file is authoritative) and reports the mismatch.
  - Entities that cannot be represented are **excluded**: malformed files, bad identifiers, an unsafe content path (the model rejects it), and an acyclic relationship pointing at its own source (`SelfReference`). Relationships to an excluded artifact then show up as dangling, which validation reports as `V0006`/`V0007` next to the cause.
  - `MissingContentFile` and a content link that resolves outside the repository (`UnsafeContentPath`, checked by canonicalizing both paths) keep the entity.
  - Dangling relationship endpoints are **not** load issues; validation finds them from the loaded entities, and the strict snapshot rejects them.
  - Files over 2 MiB and non-UTF-8 files are `MalformedFile`.
- **Init.** `init` writes `.functor/repository.toml` last and atomically. It does not create `.functor/lock`; the first writer does. The default name is the directory name, falling back to `repository`. On failure it removes only what it created.
- **Lock.** `WriteLock::run(closure)` polls `File::try_lock` every 10 ms until the timeout. `FileStore` takes the timeout from `StoreOptions` (the CLI will read `FUNCTOR_LOCK_TIMEOUT` and pass it in; libraries never read the environment).

## Implementation Notes (M7)

- **Issue context.** A `LoadIssue` carries, besides its kind, file and detail, an optional `entity` (the artifact or relationship the file defines, when it could be read that far: unknown fields, a mismatched file name, duplicates, content problems) and `related` subjects (for a duplicate id or slug, the file that defined it first). Validation turns them into `related` of the finding and uses `entity` for scoped runs.
- **Opening without parsing.** `functor_repository::discover_root` finds the repository root without reading `repository.toml`, and `FileStore::at_root` builds a store from a root alone; `validate` uses both so that a damaged `repository.toml` surfaces as `V0001`/`V0013` instead of stopping the command.

## Implementation Notes (M9)

- **Parallel loader.** Reading and parsing the entity files is done by up to 8 threads once a directory holds at least 64 candidates (`std::thread::scope`); results are put back in candidate (file name) order, and every sequential rule (duplicates, file name, content) runs afterwards, so the outcome is identical to a sequential load. Each file is opened once and read through a size limit. Loading 30,000 files (10,000 artifacts and 20,000 relationships) takes about 0.55 s; before this change it took about 1.6 s per 3,000.
- **`CachingStore`.** A decorator in `functor-store` that keeps one strict `RepositorySnapshot` for the reads `get`, `resolve`, `list` and `snapshot`, so a CLI command that reads several times loads once. `create` and `update` go to the inner store (which checks its rules against fresh state under the write lock) and then drop the cache, even when they fail. `load` (the lenient read) and `info` are never cached. A failed load is not cached. The cache lives for one command; it is wrong for a long-running process, so `watch` reads the inner store directly. It passes the same conformance suite as the stores it wraps.
- **Budgets.** What a command costs is one full load (two for `relation add`: the endpoint check and the write under the lock), which is why the budgets at 10,000 artifacts are 0.75 s and 1.5 s ([nonfunctional-requirements.md](nonfunctional-requirements.md)). Partial loading (for example only the artifacts directory for `show`) would be faster but would relax "a damaged repository is refused" for those reads; it is deferred together with the on-disk index.

## Test Obligations

- The shared store **conformance suite** runs against `MemoryStore` and `FileStore` (see [testing-strategy.md](testing-strategy.md)). Repository lifecycle tests (`init`, `open`, `discover`, lock, format version) live in `functor-repository`.
- `FileStore`-specific tests (all implemented in `functor-store/tests/file_store.rs`): canonical files named by id, in-place update preserving comments, key order, whitespace and unknown keys, a no-op update writing nothing, CRLF checkouts, every `LoadIssueKind` from damaged files, stray temp files, content path safety (including a symbolic link escaping the root where links can be created), size caps, writes refused in a damaged repository, lock contention, concurrent writers and a racing slug, moving the repository directory. Atomic replace with a failure injected before the rename is tested in `functor-repository` (`atomic_write`).
