# ADR-0007: MVP relationship type set

## Status

Accepted. Proposed amendment: [ADR-0014](ADR-0014-evidence-relationships.md) (promotes `supports`, `contradicts`, `validates`).

## Context

The relationship type set differed between documents:

- `mvp-scope.md`: `depends_on`, `references`, `derived_from`
- `cli-mvp-roadmap.md`: adds `extends`, and spelled it `depends-on` in examples
- RFC-0004: `references`, `depends_on`, `derived_from`, `extends`, `supports`, `contradicts`
- RFC-0001: `depends_on`, `derived_from`, `references`, `validates`, `supersedes`, `related_to`

## Decision

- Canonical spelling is **snake_case**: `depends_on`. The CLI accepts a kebab-case alias (`depends-on`) as input and always prints snake_case.
- The MVP **built-in** types are `depends_on`, `references`, `derived_from` and `extends`.
- Any other well-formed type name (`[a-z][a-z0-9_]*`, at most 64 characters), including `supports`, `contradicts`, `validates`, `supersedes` and `related_to`, is stored as a **custom** type. It is accepted by `relation add` only with `--allow-custom-type`, and `validate` reports it as warning `V0012`.
- `depends_on`, `derived_from` and `extends` must be acyclic (rule `V0011`). `references` may be cyclic.
- Relationships are directed: `source --type--> target`. Reverse lookups are query concerns, not stored data.

## Consequences

- The domain type is an enum with a `Custom(RelationshipTypeName)` variant, so adding a built-in type later is not a breaking change (the enum is `#[non_exhaustive]`).
- Tests must cover both spellings and the custom-type warning path.

## RFC Impact

Implements a subset of RFC-0004. Custom types keep the remaining RFC-0004 and RFC-0001 types usable without committing semantics to them.
