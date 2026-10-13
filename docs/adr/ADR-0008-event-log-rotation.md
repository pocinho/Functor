# ADR-0008: Event log rotation

## Status

Accepted. Amends [ADR-0003](ADR-0003-events-in-mvp.md).

## Context

ADR-0003 put events in the MVP but left `events/current.log` unbounded and rotation deferred. The project owner decided the MVP bounds the log.

## Decision

- Limits are fixed constants in the MVP: **10 MiB** per log file and **10 archived files** retained.
- Rotation happens under the repository write lock, immediately before appending an event: if `events/current.log` is at or above 10 MiB, it is renamed to `events/archive/events-<first-seq>-<last-seq>.log` (sequence numbers zero-padded to 12 digits), and the new event is written as the first line of a fresh `current.log`. `current.log` is therefore never empty after a rotation, and `seq` continues across files.
- After a successful rotation, archives beyond the 10 newest (by sequence) are deleted. Pruning loses history by design; events remain informational and non-authoritative (RFC-0017).
- When `current.log` is missing, the next `seq` is the last sequence in the newest archive file name plus 1, or 1 when there are no archives.
- `watch` follows rotation: if the current file is replaced or is shorter than the saved offset, it continues from the start of the new file. `--after <seq>` and `--from-start` read archives in order when the requested events are in them; events that were pruned are absent and a warning says so.
- Making the limits configurable is deferred.

## Consequences

- At most about 110 MiB of event data per repository.
- Rotation and pruning add failure points; a failure to rotate or prune is a warning and does not block the append (the file simply grows past the limit until the next successful rotation).
- Tests must cover `seq` continuity across rotation, pruning, a crash between rename and the first write, and watcher behavior during rotation.

## RFC Impact

Narrows the deviation in [../rfc-conformance.md](../planning/rfc-conformance.md) (C-0017-6): auditability is bounded to the retained window rather than absent.
