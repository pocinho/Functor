# Domain Model

Status: Draft
Related RFCs: RFC-0001, RFC-0003, RFC-0004, RFC-0005, RFC-0006, RFC-0007, RFC-0015, RFC-0017
Owner crate: `functor-model`
Related ADRs: ADR-0001, ADR-0005, ADR-0007

## Purpose

This document specifies the Rust-level types and invariants of `functor-model`. It is the contract that `functor-core`, `functor-store`, `functor-query`, `functor-validation`, `functor-events` and `functor-cli` build on.

## Rules for the Model Crate

- Pure types and logic only. No I/O, no async runtime, no `serde` (ADR-0005).
- Allowed dependencies: `ulid` (id generation and parsing) and `thiserror`. Anything else needs a note in [dependency-policy.md](dependency-policy.md).
- Invariants are enforced by **validating constructors** (`parse`, `new`). Fields are private; accessors are provided. An invalid value cannot be constructed.
- Public enums that may grow are `#[non_exhaustive]`.
- All types implement `Debug`, `Clone`, `PartialEq`, `Eq` and `Hash` where meaningful, and `Display` where there is a canonical textual form.
- Time values are `Timestamp`, a wrapper around UTC time at second precision. The clock is injected (see `Clock` in `functor-core`) so tests are deterministic.

## Identifiers

```rust
pub struct ArtifactId(Ulid);        // text: "art_<26-char ULID>"
pub struct RelationshipId(Ulid);    // "rel_<ULID>"
pub struct RepositoryId(Ulid);      // "repo_<ULID>"
pub struct EventId(Ulid);           // "evt_<ULID>"
```

- `FromStr`/`Display` round-trip exactly. Parsing rejects wrong prefixes, wrong length and lowercase ULID text.
- Generation takes an injected id source so tests can be deterministic.
- Ids are opaque. No accessor exposes the timestamp part (RFC-0005: consumers must not infer meaning). Ordering by id is allowed only for stable listing.

```rust
pub struct Slug(String);            // grammar in identity-and-addressing.md
pub enum ArtifactRef {              // what a user can type
    Id(ArtifactId),
    Slug(Slug),
}
```

`ArtifactRef::parse` returns `Id` if the text starts with `art_` and parses as an id, otherwise it tries `Slug`. Slugs may never start with `art_` (see [identity-and-addressing.md](identity-and-addressing.md)), so the two forms never collide.

## Artifact

```rust
pub struct Artifact {
    id: ArtifactId,
    slug: Slug,
    artifact_type: ArtifactType,
    title: Title,                  // non-empty, trimmed, <= 200 chars, no control characters
    revision: Revision,            // starts at 1
    description: Option<String>,
    tags: Tags,                    // sorted, de-duplicated, each [a-z0-9][a-z0-9_-]{0,39}
    metadata: Metadata,            // BTreeMap<MetadataKey, MetadataValue>
    content: Content,
    created: Timestamp,
    updated: Timestamp,
}

pub struct ArtifactType(String);   // [a-z][a-z0-9_]{0,31}; well-known values below
pub struct Revision(NonZeroU64);

pub enum Content {
    None,
    Inline { format: ContentFormat, text: String },
    Path { format: ContentFormat, path: ContentPath },   // relative, normalized, no ".."
}
pub struct ContentFormat(String);  // e.g. "markdown", "toml", "text"
```

- `ArtifactType` is an open set (RFC-0003: implementations may add types). Well-known constants: `document`, `rfc`, `note`, `specification`, `dataset`, `workflow`, `source`, `report`. The set is advisory; validation never rejects an unknown type.
- `Metadata` is extensible and never alters semantics (RFC-0003). Keys follow the tag grammar. Values are strings, integers, booleans or arrays of strings (a closed `MetadataValue` enum).
- `ContentPath` is relative to the repository root, uses `/` separators, cannot be absolute and cannot contain `..`. Existence is checked by validation (`V0010`), not by the constructor, because the model does no I/O.
- Updating an artifact produces a new `Artifact` with `revision + 1` and a new `updated` timestamp. Mutable fields change only through `ArtifactPatch`. `id`, `artifact_type` and `created` never change.

```rust
pub struct NewArtifact { slug, artifact_type, title, description, tags, metadata, content }
pub struct ArtifactPatch { slug: Option<Slug>, title: Option<Title>, description: Option<Option<String>>, tags: Option<Tags>, metadata: Option<Metadata>, content: Option<Content> }
```

`ArtifactPatch` deliberately has no `artifact_type` field: changing the type is deferred.

## Relationship

```rust
pub struct Relationship {
    id: RelationshipId,
    relationship_type: RelationshipType,
    source: ArtifactId,
    target: ArtifactId,
    created: Timestamp,
    metadata: Metadata,
}

#[non_exhaustive]
pub enum RelationshipType {
    DependsOn,
    References,
    DerivedFrom,
    Extends,
    Custom(RelationshipTypeName),   // [a-z][a-z0-9_]{0,63}
}
```

- Text form is snake_case. `RelationshipType::parse` also accepts kebab-case (`depends-on`) and normalizes. `Display` always emits snake_case.
- A built-in name always parses to the built-in variant, never to `Custom`. Constructing `Custom("depends_on")` directly is rejected.
- Relationships store **ids**, never slugs or paths.
- Constructors reject self-reference for the acyclic types (`depends_on`, `derived_from`, `extends`). Other cycle shapes are detected by validation because they need the whole graph.
- `NewRelationship { relationship_type, source, target, metadata }`. Existence of source and target is **not** checked by the model constructor, but `RelationshipRepository::create` rejects a dangling endpoint (`FunctorError::NotFound`) so the CLI cannot create invalid state. Hand-edited or merged files can still be dangling, which is why `validate` exists.
- Relationships are immutable in the MVP. There is no update and no remove (deferred; no `relationship.deleted` event).

## Repository

```rust
pub struct RepositoryInfo {
    id: RepositoryId,
    name: RepositoryName,          // non-empty, trimmed, <= 100 chars
    description: Option<String>,
    format_version: FormatVersion, // u32, MVP = 1
    created: Timestamp,
}

pub struct RepositorySnapshot {
    info: RepositoryInfo,
    artifacts: Vec<Artifact>,          // sorted by ArtifactId
    relationships: Vec<Relationship>,  // sorted by RelationshipId
}
```

A snapshot is an immutable, consistent in-memory view used by query and validation. It offers pure indexed lookups (`by_id`, `by_slug`, `outgoing`, `incoming`) built once at construction. Duplicate ids or slugs are not representable in a well-formed snapshot; the lenient loader reports them as load issues instead (see `SnapshotLoad` in [traits-and-interfaces.md](traits-and-interfaces.md)).

## Query Types

Defined in detail in [query-model-mvp.md](query-model-mvp.md).

```rust
pub enum Query { Lookup(ArtifactRef), Search(SearchQuery), Traverse(Traversal) }
pub struct SearchQuery { artifact_type: Option<ArtifactType>, tag: Option<Tag>, text: Option<String>, metadata: Vec<(MetadataKey, MetadataValue)> }
pub struct Traversal { start: ArtifactRef, relationship_types: Vec<RelationshipType>, direction: Direction, max_depth: Option<u32> }
pub enum Direction { Outbound, Inbound, Both }
```

## Validation Types

Defined in detail in [validation-rules.md](validation-rules.md).

```rust
pub struct RuleCode(&'static str);          // "V0003"
pub enum Severity { Error, Warning, Info }
pub enum Outcome { Passed, Failed, Warning, Unknown }   // RFC-0007

pub struct Finding {
    code: RuleCode,
    severity: Severity,
    subject: Subject,           // Artifact(id) | Relationship(id) | Repository | File(path)
    message: String,
}

pub struct ValidationReport { outcome: Outcome, findings: Vec<Finding>, checked: Counts, completed: Timestamp }
```

`Outcome` derivation: any `Error` finding gives `Failed`; otherwise any `Warning` gives `Warning`; otherwise `Unknown` if any rule could not run; otherwise `Passed`. Findings are sorted deterministically (code, then subject, then message).

## Event Types

Defined in detail in [event-model-mvp.md](event-model-mvp.md).

```rust
#[non_exhaustive]
pub enum EventKind { ArtifactCreated, ArtifactUpdated, RelationshipCreated, ValidationCompleted }
pub struct Event { id: EventId, seq: u64, at: Timestamp, kind: EventKind, origin: RepositoryId, subject: Subject, data: EventData }
```

## Ordering and Equality

- Artifacts and relationships compare by value. Identity comparison uses ids only.
- Every collection returned by the API has a **documented, deterministic order**. Default order for lists: by id ascending. Queries document their own ordering.
- `HashMap`/`HashSet` iteration order must never reach a user-visible output; use `BTreeMap`/sorted vectors.

## Invariant Summary (testable)

| Invariant | Test type |
|-----------|-----------|
| Id text round-trips for every id type | property |
| Slug grammar accepts and rejects the documented sets | unit table |
| `RelationshipType::parse(display(t)) == t` and kebab alias normalizes | property |
| Built-in names never become `Custom` | unit |
| `Revision` can never be zero | unit |
| Tags are sorted and unique after construction | property |
| `ContentPath` rejects absolute paths and `..` | unit table |
| Report outcome derivation follows the rule above | unit table |
