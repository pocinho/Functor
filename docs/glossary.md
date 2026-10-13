# Glossary

Canonical terms for the Functor CLI MVP. When docs and code disagree with this glossary, fix the doc or the code.

| Term | Meaning | Notes |
|------|---------|-------|
| **Artifact** | The primary durable unit (RFC-0003). Has an id, type, revision, metadata and content. | Not "document" or "file". |
| **Artifact id** | Opaque, immutable identifier `art_<ULID>` (RFC-0005). | See [identity-and-addressing.md](design/identity-and-addressing.md). |
| **Slug** | Unique human-friendly alias of an artifact within a repository. Renamable. Not identity. | Example: `rfc-0000`. |
| **Artifact type** | Classifies an artifact (RFC-0003 "Type"). Serialized as the `type` field; the Rust field is `artifact_type` because `type` is a keyword. | Earlier docs called it `kind`. |
| **Revision** | Integer starting at 1 that increments on each update. The MVP form of RFC-0003 "Version". | Full versioning (RFC-0016) is deferred. |
| **Relationship** | A first-class, directed, identified edge `source --type--> target` (RFC-0004). | CLI noun is `relation` for brevity. |
| **Relationship type** | Names the semantics of an edge, for example `depends_on`. | See ADR-0007. |
| **Source / target** | The two ends of a relationship. `source` is the artifact that holds the dependency. | `rfc-0001 depends_on rfc-0000`: source is rfc-0001. |
| **Dependencies of X** | Targets reachable from X by following `depends_on` outwards. | |
| **Dependents of X** | Sources that reach X by following `depends_on` inwards (reverse). | |
| **Repository** | The bounded collection of artifacts, relationships and metadata rooted at a directory containing `.functor/` (RFC-0015). | The vision documents say "workspace" for the same idea at a conceptual level. |
| **Repository root** | Directory that contains `.functor/repository.toml`. | |
| **Projection** | A user-facing experience built over the core API (RFC-0002, RFC-0011). The CLI is one. | |
| **Core API** | The traits in `functor-core`. | |
| **Composition root** | The place that picks concrete implementations and wires them. In the MVP, `functor-cli`. | |
| **Store** | An implementation of the persistence traits (`FileStore`, `MemoryStore`). | |
| **Validation** | A check of repository properties producing a report (RFC-0007). | |
| **Finding** | One validation result with a rule code, severity, subject and message. | |
| **Outcome** | `Passed`, `Failed`, `Warning` or `Unknown` (RFC-0007). | |
| **Event** | An informational record of a state change (RFC-0017). Never authoritative. | |
| **Canonical reference** | `functor:<entity-type>:<id>` (RFC-0005). | |
| **Format version** | Integer in `repository.toml` identifying the on-disk layout. | MVP value: `1`. |
| **Golden file** | Expected CLI output stored in the repository and compared by tests. | |
| **ADR** | Architecture Decision Record. See [adr/README.md](adr/README.md). | |
