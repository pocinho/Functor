# Event Model (MVP)

Status: Accepted
Related RFCs: RFC-0017 (primary), RFC-0015
Related ADRs: ADR-0003
Owner crate: `functor-events`
Related docs: [traits-and-interfaces.md](../architecture/traits-and-interfaces.md), [storage-design.md](storage-design.md), [cli-spec.md](../cli/cli-spec.md)

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
| `run.started` | run orchestrator | run artifact id | `workflow`, `provider`, `request_id` |
| `run.completed` | run orchestrator (also `run recover`) | run artifact id | `outcome`, `result`, `outputs`, `truncated` |
| `validation.recorded` | run orchestrator, `validate --save-report` | report artifact id | `outcome`, `subject`, `errors`, `warnings`, `infos` |

Run and validation events follow the same rules as the others: published after the durable transition, a failed append is a warning ([execution-and-evidence.md](execution-and-evidence.md), [ADR-0015](../adr/ADR-0015-validation-report-persistence-and-events.md)). Evidence artifacts do not emit `artifact.created`.

Not in the MVP: `artifact.deleted`, `relationship.deleted`, `repository.changed`, `repository.created` (an earlier workspace doc listed `repository.changed`; it is dropped because it overlaps the specific events) and capability events.

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
- If the last line is incomplete (no trailing newline, a crash artifact), readers skip it with a warning and the writer terminates it with a newline before appending.
- **Rotation** ([ADR-0008](../adr/ADR-0008-event-log-rotation.md)): under the same lock, immediately before appending, if `current.log` is at or above 10 MiB it is renamed to `events/archive/events-<first-seq>-<last-seq>.log` (sequence numbers zero-padded to 12 digits) and the new event becomes the first line of a fresh `current.log`. Archives beyond the 10 newest are then deleted. A failure to rotate or prune is a warning; the append still happens. The retained window is at most about 110 MiB.
- When `current.log` is missing, the next `seq` is the last sequence in the newest archive file name plus 1, or 1 if there are no archives.

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
4. The watcher reads the log file by byte offset and keeps the offset; it handles the log being created after `watch` starts, and rotation or truncation (the file was replaced, or the offset is greater than the size: continue from the start of the new file). Rotation is detected by comparing the first line's `seq` and the file length with the saved state; no event is delivered twice because delivery is keyed by `seq`.
5. A partial last line is held back until its newline arrives.
6. Filters: `--type <event type>` (repeatable, prefix match on `artifact.`) and `--subject <ref>` (artifact reference resolved at start). No other filtering in the MVP.
7. Output: `text` one line per event; `json` one JSON object per line (the envelope as stored, so the output is the log). Output is flushed after every event so pipes work. Spec in [cli-spec.md](../cli/cli-spec.md).
8. `watch` never takes the write lock, never writes, and never modifies state.
9. Termination: Ctrl-C (SIGINT, and on Windows the console control event) exits 0. A closed output pipe exits 0 silently. An unreadable log yields exit code 7. `--count <n>` exits after N events (used by tests) and `--timeout <seconds>` exits 0 after the duration with no more events (also used by tests).

When the start position (`--after N` or `--from-start`) refers to events that were already pruned from the archives, `watch` starts at the oldest retained event and prints `warning: events before seq <S> were pruned`.

Multiple watchers are allowed. `watch` works across processes because the log file is the integration point; there is no daemon and no IPC.

## Security and Privacy

The log records slugs and ids, not content. It contains no secrets by design. RFC-0013 trust semantics (signed events) are deferred.

## Deferred (RFC-0017)

Durable subscriptions with named cursors, replay APIs, event schemas published for third parties, delivery guarantees stronger than above, cross-repository streams, configurable rotation limits and compaction, and deletion events.

## Test Obligations

- Each mutating command emits exactly one event of the right type and data; read-only commands emit none (including failed commands).
- `seq` is strictly increasing under concurrent writers (two processes, many appends) with no duplicates and no gaps.
- Event append failure (read-only log) yields a warning and exit 0, with state committed.
- Watcher tests using `--count` and `--timeout`: existing events, new events after start, log created after start, `--after`, truncated tail line, filters, rotation while watching, start position before the oldest retained event.
- Rotation: `seq` continues across `current.log` and archives, exactly 10 archives are retained, the oldest is pruned, the new `current.log` is never empty after a rotation, a crash between the rename and the first write is recovered on the next append, a rotation or prune failure only warns.
- Envelope round trip and unknown-key tolerance.

## Implementation Notes (M8)

- **Crate shape.** `functor-events` has the wire format (`encode_event`, `decode_event`, `peek_seq`, `LineProblem`), `FileEventLog` (publisher and event source; rotation limits and lock timeout in `EventLogOptions`) and `Follower` (what `watch` uses). `FileEventLog::publish_detailed` returns the event with its rotation and pruning warnings; the core `EventPublisher::publish` drops the warnings. The `EventSource` implementation hides a follower's warnings behind the plain `EventSubscription` trait, which has no way to carry them; `watch` uses `Follower` directly and prints them.
- **Numbering.** The next `seq` is the highest `seq` found on the last lines of `current.log` (looking back past unusable lines, in growing windows) or in the newest archive name, plus one. A torn last line that is complete JSON still reserves its number. A torn or garbage line is terminated with a newline before the next event, and readers skip it with a warning.
- **Wire reading.** Lines with an unknown envelope version are ignored with one warning per version; lines with an unknown event type are ignored silently; both still consume their sequence number. Unknown keys and unknown value shapes in `data` are ignored.
- **Following.** A `Follower` keeps the highest `seq` delivered, the byte offset in `current.log` and the identity of the file it is reading: the `seq` and event id of its first line. Each poll first reads archives that hold events not yet delivered, then `current.log`; a changed first line, a missing file or a shorter file means the file was replaced, and the poll goes around again so events that moved into a new archive are not skipped. The same applies the first time a file is seen with a first `seq` that leaves a gap after what was delivered: a rotation may have completed since the archives were listed, and the archive that fills the gap exists by then, so the archives are read again before the file (without this, a busy publisher rotating many times could make a watcher lose a whole archive; a test stresses it and a deterministic test pins the order). Only complete lines are consumed, so a half-written event waits for its newline.
- **Numbering restarts.** If the log is deleted or truncated and numbering restarts at or below what was already delivered, the follower treats the new file as a reset and delivers it from its start, instead of silently dropping events as already seen. The file identity includes the first event's id because a reset log can start at the same `seq` with the same line length.
- **Waiting.** With notifications (`notify`, a recursive watch on `events/`) the follower wakes on a change and polls at least every 5 seconds; without them (the platform refuses, or `events/` does not exist yet) it polls every 500 ms. The idle timeout is part of the wait, and `next_within` is cancel-safe only while it waits.
- **Limits.** The 10 MiB and 10 archive limits are the defaults of `Limits`; making them user configurable remains deferred, but the type lets tests use small values.
- **Dependencies.** `notify` is CC0-1.0 licensed and listed in `deny.toml` ([dependency-policy.md](../architecture/dependency-policy.md)).
