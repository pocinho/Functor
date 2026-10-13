# ADR-0003: Events are part of the MVP

## Status

Accepted. Amended by [ADR-0008](ADR-0008-event-log-rotation.md) (log rotation is in the MVP).

## Context

`cli-mvp-roadmap.md` included a Phase 8 with an RFC-0017 subset and `functor watch`. `mvp-scope.md` and `docs/README.md` listed events as excluded. The project owner decided the minimal event subset is in the MVP.

## Decision

The MVP includes:

- Event types: `artifact.created`, `artifact.updated`, `relationship.created`, `validation.completed`.
- Persistence as JSON Lines in `events/current.log`.
- `functor watch`, which streams events as they are appended.
- A new crate, `functor-events`, plus the `EventPublisher` trait in `functor-core`.

Events are **informational, not authoritative**. Repository state is always reconstructible from artifacts and relationships. A failed event append must not roll back a committed state change; it is reported as a warning.

Event ordering rule: commit the state change first, then append the event.

Everything else in RFC-0017 (subscriptions with a filtering language, replay, streams across repositories, delivery guarantees, `*.deleted` events and so on) stays deferred. See [../event-model-mvp.md](../event-model-mvp.md).

## Consequences

- Event emission is in the critical path of every mutating command and must be tested.
- `watch` is the only long-running command and needs a clean cancellation story.
- The MVP workspace gains `functor-events` (see ADR-0004 and [../workspace-layout.md](../workspace-layout.md)).

## RFC Impact

Partial implementation of RFC-0017. Deviations (no replay, no archive rotation) are recorded in [../rfc-conformance.md](../rfc-conformance.md).
