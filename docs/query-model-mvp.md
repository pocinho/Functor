# Query Model (MVP)

Status: Draft
Related RFCs: RFC-0006 (primary), RFC-0004, RFC-0005
Owner crate: `functor-query`
Related docs: [traits-and-interfaces.md](traits-and-interfaces.md), [domain-model.md](domain-model.md), [cli-spec.md](cli-spec.md)

## Purpose

Specifies the subset of RFC-0006 implemented by the MVP: which operations exist, their exact semantics, result shapes and ordering. The CLI `query` command is a thin wrapper over this model.

## RFC-0006 Coverage

| RFC operation | MVP status | Realized as |
|---------------|-----------|-------------|
| Lookup | Implemented | `QueryEngine::lookup(ArtifactRef)` |
| Search | Subset | `SearchQuery`: type, tag, metadata equality, case-insensitive substring on slug/title/description |
| Traverse | Implemented | `Traversal`: direction, relationship types, depth limit |
| Filter | Subset | Same fields as Search plus relationship-type filters on traversals |
| Project | Projection concern | The CLI renders fixed text columns and full JSON documents; there is no `--fields` option in the MVP |
| Query language syntax | Deferred | No textual query language in the MVP |
| Content search | Deferred | Search does not read referenced content files |
| Ranking, federated, reasoning | Deferred | |

## Execution Model

1. The engine obtains a `RepositorySnapshot` (strict load) from a `RepositoryReader`.
2. It runs a **pure, synchronous** function over the snapshot.
3. Results are owned values with documented order.

A query sees one consistent snapshot for its whole execution (RFC-0006: results remain immutable during a query). There are no caches across calls.

Snapshot indexes: `by_id`, `by_slug`, outgoing and incoming adjacency lists keyed by `(artifact id)` holding relationships sorted by relationship id.

## Lookup

`lookup(reference)` resolves an id, canonical reference or slug (see [identity-and-addressing.md](identity-and-addressing.md)) and returns `Option<Artifact>`. The CLI turns `None` into `NotFound` with suggestions.

## Search

```rust
pub struct SearchQuery {
    pub artifact_type: Option<ArtifactType>,
    pub tag: Option<Tag>,
    pub text: Option<String>,                       // case-insensitive substring
    pub metadata: Vec<(MetadataKey, MetadataValue)> // all must match (AND)
}
```

Semantics:

- All present criteria are combined with AND. No criteria returns every artifact.
- `text` matches slug, title or description using Unicode simple case folding.
- `tag` matches when the artifact's tag set contains the tag.
- `metadata` matches when the key exists and its value equals the given value; for array values it matches when the array contains the given string.
- **Order:** by slug ascending (byte order), then id.

## Traverse

```rust
pub struct Traversal {
    pub start: ArtifactRef,
    pub relationship_types: Vec<RelationshipType>,   // empty = all types
    pub direction: Direction,                        // Outbound | Inbound | Both
    pub max_depth: Option<u32>,                      // None = unlimited
}

pub struct TraversalResult {
    pub start: Artifact,
    pub nodes: Vec<TraversalNode>,        // excludes the start node
    pub edges: Vec<Relationship>,         // every distinct edge examined within the depth limit
}
pub struct TraversalNode { pub artifact: Artifact, pub depth: u32, pub via: Relationship }
```

Algorithm: breadth-first search from `start`.

- Edges are followed **outbound** (`source -> target`) for `Outbound`, **inbound** (`target -> source`) for `Inbound`, or both for `Both`.
- A node is reported once, at its **minimum** depth. `via` is the first edge that reached it, with each frontier's edges taken in ascending relationship-id order so that the choice is deterministic.
- Cycles terminate: visited nodes are never re-expanded. The start node is never reported as a result node even if a cycle returns to it.
- `max_depth = Some(1)` returns direct neighbours only. `Some(0)` returns no nodes. `None` returns the whole reachable closure.
- Only relationships whose type is in `relationship_types` are followed (all if empty).
- `edges` lists each traversed edge once, sorted by relationship id, including edges to already-visited nodes within the depth limit (so the cycle structure remains visible to callers).
- **Order of `nodes`:** by depth ascending, then slug ascending, then id.
- A relationship whose endpoint is missing cannot exist in a strict snapshot, so traversal never needs to handle dangling edges.

## Named Queries Used by the CLI

| CLI | Engine call |
|-----|-------------|
| `query artifacts [filters]` | `search` |
| `query dependencies <a>` | `traverse(start=a, types=[depends_on], Outbound, max_depth=--depth)` |
| `query dependents <a>` | `traverse(start=a, types=[depends_on], Inbound, max_depth=--depth)` |
| `query relationships <a>` | all relationships touching `a`, in both directions, optionally filtered by `--type` and `--direction`, sorted by relationship id |

Semantics of the two directions, using `rfc-0001 depends_on rfc-0000` (source `rfc-0001`, target `rfc-0000`):

- `dependencies rfc-0001` returns `rfc-0000` (what rfc-0001 needs).
- `dependents rfc-0000` returns `rfc-0001` (what needs rfc-0000).

`--depth` defaults to `1` (direct neighbours); `--depth all` is the transitive closure. `--type` can replace the default `depends_on` (for example `--type derived_from`, repeatable).

## Result Contract (RFC-0006)

A result always carries the artifact identity (`id`, `slug`), the relationship information when applicable (`via`, `edges`) and the metadata needed by the projection. The CLI JSON schemas are in [cli-spec.md](cli-spec.md).

## Errors

- Unknown reference: `NotFound`.
- Invalid `max_depth` or malformed filter input is a CLI usage error (exit 2) before the engine is called.
- An inconsistent repository (duplicate ids, dangling relationships) makes the strict snapshot fail with `Inconsistent`; the user is told to run `functor validate`.

## Performance

Building a snapshot is O(N) in files. Traversal is O(V + E) over the requested closure. Targets are in [nonfunctional-requirements.md](nonfunctional-requirements.md). No on-disk index in the MVP.

## Test Obligations

- Table-driven traversal tests: chain, diamond, cycle, self-loop attempt, disconnected nodes, depth limits 0, 1, n and unlimited, each direction, type filters.
- Property: traversal terminates on any generated graph, reports each node once, and `Inbound` of `Outbound` neighbours includes the start.
- Search: every filter alone and in combination; ordering.
- Same results from `MemoryStore` and `FileStore` snapshots.
