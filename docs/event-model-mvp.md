# Event Model (MVP)

Status: Draft
Related RFCs: RFC-0017 (primary), RFC-0015
Related ADRs: ADR-0003
Owner crate: `functor-events`
Related docs: [traits-and-interfaces.md](traits-and-interfaces.md), [storage-design.md](storage-design.md), [cli-spec.md](cli-spec.md)

## Purpose

Specifies the minimal event subset in the MVP (ADR-0003): event types, envelope, log format, ordering rules, and the semantics of `functor watch`.

## Principles

- Events are **informational records**, not authoritative state. Deleting the log never loses artifacts or relationships.
- State changes commit first; the event is appended after (ADR-0003). A failed append is a **warning**, not an error, and never rolls back state.
- Events are immutable and append-only.
- Delivery is **at most once** from the writer's perspective, and **at least once** to a watcher that persists its last seen sequence number (restart with `--after <seq>`). There is no replay guarantee for events that failed to append.

## Event Types

| Type | Emitted by | Subject | Data |
|------|-----------|---------|------|
| `artifact.created` | `artifact create` | artifact id | `slug`, `type`, `revision` |
| `artifact.updated` | `artifact update` | artifact id | `slug`, `revision`, `changed` (list of changed fields) |
| `relationship.created` | `relation add` | relationship id | `relationship_type`, `source`, `target` |
| `validation.completed` | `validate` | repository id | `outcome`, `errors`, `warnings`, `infos`, `strict` |

Not in the MVP: `artifact.deleted`, `relationship.deleted`, `repository.changed`, `repository.created` (an earlier workspace doc listed `repository.changed`; it is dropped because it overlaps the specific events), execution and capability events.

`init` does not emit an event: the log does not exist yet and the first event is the first change after creation.

## Envelope

JSON object, one per line (JSON Lines), UTF-8, keys in this order:

```json
{"v":1,"id":"evt_01J9Z3KRA0B1C2D3E4F5G6H7J8","seq":3,"at":"2026-10-12T18:05:00Z","type":"relationship.created","origin":"functor:repository:repo_01J9Z3JX9T8S7R6Q5P4N3M2K1H","subject":"functor:relationship:rel_01J9Z3KQ0A1B2C3D4E5F6G7H8J","data":{"relationship_type":"depends_on","source":"art_01J9Z3K8M2Q7X4V6B5N1R0T8WD","target":"art_01J9Z3K0P4Q5R6S7T8V9W0X1Y2"},"metadata":{}}
```

| Field | Notes |
|-------|-------|
| `v` | Envelope version, currently `1`. Readers ignore events with an unknown `v` and warn once |
| `id` | `evt_<ULID>`, unique |
| `seq` | Positive integer, strictly increasing within a repository, starting at 1, no gaps in a single log file |
| `at` | UTC timestamp, second precision, from the injected clock |
| `type` | Dotted name from the table above |
| `origin` | Canonical reference of the repository that produced the event (RFC-0017 "Origin") |
| `subject` | Canonical reference of the entity the event is about |
| `data` | Object, per-type fields above. Readers ignore unknown keys |
| `metadata` | Object for additional metadata (RFC-0017 "Metadata"); always `{}` in the MVP, reserved for extensions |

The envelope satisfies the RFC-0017 requirement that every event contains an identifier, type, timestamp, origin, subject and metadata. `seq`, `v` and `data` are MVP additions. `functor:repository:<id>` is the canonical reference form of [identity-and-addressing.md](identity-and-addressing.md); the event DTO does not otherwise interpret it.

The event wire DTO lives in `functor-events` (ADR-0005).

## Log File

- Path: `events/current.log` (see [repository-layout.md](repository-layout.md)).
- Append-only; each event is written with a single `write` of `json + "\n"` to a file opened with `O_APPEND` semantics, then flushed.
- **Sequence assignment:** under the repository write lock (same lock as state writes, see [storage-design.md](storage-design.md)), read the last complete line's `seq` and write `seq + 1`. Reading the tail is O(1): seek back from the end to the previous newline.
- If the log is missing, the next `seq` is 1. If the last line is incomplete (no trailing newline, a crash artifact), readers skip it with a warning and the writer terminates it with a newline before appending.
- Rotation to `events/archive/` is **not** implemented. The log grows without bound in the MVP; acceptable for the target scale, recorded in [rfc-conformance.md](rfc-conformance.md).

## Publishing

`EventPublisher::publish(NewEvent) -> Event` assigns `id`, `seq` and `at`. The CLI command flow for a mutation:

1. Perform the state change through the repository traits (commits atomically).
2. Call `publish`. On failure, print `warning: event not recorded: <reason>` to stderr and continue; exit code stays 0.
3. Print the command result.

`validate` always publishes `validation.completed`, including when the outcome is `Failed`. It does not publish if the run failed to start.

## Watching: `functor watch`

`EventSource::subscribe(position)` returns a subscription whose `next()` yields events in `seq` order.

Behavior:

1. Resolve the start position: `--after <seq>` (events with `seq > N`), `--from-start`, or by default the **end** of the log (only new events).
2. Deliver existing events, if any, then wait for new ones.
3. Waiting uses filesystem notification (`notify` crate) on `events/`, with a **polling fallback** every 500 ms, and a poll even when notifications are active every 5 seconds as a safety net (notification APIs miss events on some network and virtual filesystems).
4. The watcher reads the log file by byte offset and keeps the offset; it handles the log being created after `watch` starts, and truncation or replacement (offset greater than size: restart from the new start with a warning).
5. A partial last line is held back until its newline arrives.
6. Filters: `--type <event type>` (repeatable, prefix match on `artifact.`) and `--subject <ref>` (artifact reference resolved at start). No other filtering in the MVP.
7. Output: `text` one line per event; `json` one JSON object per line (the envelope as stored, so the output is the log). Output is flushed after every event so pipes work. Spec in [cli-spec.md](cli-spec.md).
8. `watch` never takes the write lock, never writes, and never modifies state.
9. Termination: Ctrl-C (SIGINT, and on Windows the console control event) exits 0. A closed output pipe exits 0 silently. An unreadable log yields exit code 7. `--count <n>` exits after N events (used by tests) and `--timeout <seconds>` exits 0 after the duration with no more events (also used by tests).

Multiple watchers are allowed. `watch` works across processes because the log file is the integration point; there is no daemon and no IPC.

## Security and Privacy

The log records slugs and ids, not content. It contains no secrets by design. RFC-0013 trust semantics (signed events) are deferred.

## Deferred (RFC-0017)

Durable subscriptions with named cursors, replay APIs, event schemas published for third parties, delivery guarantees stronger than above, cross-repository streams, log rotation and compaction, and deletion events.

## Test Obligations

- Each mutating command emits exactly one event of the right type and data; read-only commands emit none (including failed commands).
- `seq` is strictly increasing under concurrent writers (two processes, many appends) with no duplicates and no gaps.
- Event append failure (read-only log) yields a warning and exit 0, with state committed.
- Watcher tests using `--count` and `--timeout`: existing events, new events after start, log created after start, `--after`, truncated tail line, filters.
- Envelope round trip and unknown-key tolerance.
