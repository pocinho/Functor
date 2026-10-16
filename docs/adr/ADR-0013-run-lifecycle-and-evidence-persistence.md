# ADR-0013: Run lifecycle, input capture and evidence persistence

## Status

Accepted

## Context

Storage writes one entity per atomic operation under a short repository lock ([storage-design.md](../design/storage-design.md)); there is no multi-entity transaction and `revision` cannot recover earlier state. A run touches many entities and a child process that may take minutes. Evidence must survive restarts, expose interruption honestly and never be confused with a successful publication.

## Decision

1. **Evidence is artifacts.** A run is represented by canonical artifacts, queryable through normal APIs, with `format_version` 1 unchanged. Reserved artifact types, each carrying `metadata.evidence = "<schema>"`:

   | Type | Role | Mutable |
   |------|------|---------|
   | `capture` | Exact bytes of one workflow or input at run time (digest, size, source id and revision) | No |
   | `run` | Run intent: provider, workflow capture, input captures, configuration, limits, environment names, program provenance | No |
   | `run_result` | Terminal record: outcome, timestamps, exit status, truncation flags, output artifact ids | No |

   Evidence artifacts are append-only. The shared store rules refuse `update` of an artifact that has an evidence `type` **and** the `evidence` metadata key with `Conflict` ("evidence is immutable"). A pre-existing artifact that merely uses one of these type names is unaffected.
2. **Capture.** Workflow and input bytes are read through a bounded read (limits in the contract), hashed with SHA-256 (`sha256:<hex>`), and stored as a content-addressed file `content/evidence/captures/<hex>` written create-new, referenced by a `capture` artifact's content path. Consistency: read the artifact record, read the bytes, re-read the record; if `revision` or the bytes' digest changed, retry once, then fail `Conflict` (`concurrent input change`) before any run intent exists. Identical bytes share one file.
3. **Run lifecycle.** Persisted states are derived from which immutable records exist:

   ```text
   (none) -> intent_recorded -> result_recorded(outcome)
   ```

   Terminal outcomes in `run_result`: `succeeded`, `failed` (non-zero exit or declared output missing/invalid), `timed_out`, `cancelled`, `start_failed` (program or provider could not start), `interrupted` (written only by recovery). An intent with no result is reported as `running` when its per-run lock is held and `interrupted_unknown` when it is not. The names were approved with this ADR.
4. **Outcome is not error.** A completed child with a bad outcome returns `Ok(RunRecord)` with that outcome. `Err(FunctorError)` means infrastructure or policy failure: `PermissionDenied`, `Unsupported`, `InvalidInput`, `Storage`, `Locked`, `Conflict`. A validation outcome is a separate value ([ADR-0015](ADR-0015-validation-report-persistence-and-events.md)).
5. **Publication order** (every step is its own short write lock; the lock is never held while the child runs):
   1. Capture files, then `capture` artifacts.
   2. `run` artifact.
   3. Intent relationships (below).
   4. Acquire the **per-run lock** `.functor/runs/<run-id>.lock` (std `File::try_lock`, held for the child's lifetime). Any failure in steps 1 to 4 means **no child starts**; the error is returned and orphaned captures are reported by validation, not deleted.
   5. Spawn, wait, enforce limits.
   6. Output artifacts (one write each) and their content, from `outputs/<name>`.
   7. `run_result` (the **commit point**, listing output ids).
   8. Result relationships.
   9. Release the per-run lock, then publish `run.completed`.

   A run is complete if and only if its `run_result` exists. Outputs without a result are orphans, never a success. If step 6 or 7 fails the call returns `Storage` carrying the run id; the staged `.functor/runs/<run-id>/` directory is kept.
6. **Relationships are repairable.** Missing relationships (steps 3 and 8) are derivable from the recorded ids. Recovery adds only relationships whose `(source, type, target)` is absent and provably implied by an existing record.
7. **Recovery** (`run recover`, library `recover_runs`): for each intent without a result, try the per-run lock. Held means still running; leave it. Acquired means the owner is gone: write `run_result { outcome = interrupted }`, repair implied relationships, and leave staged files in place. Recovery **never reruns** a child, never publishes unpublished outputs, and never deletes evidence. Opening a repository never recovers automatically; reads report interrupted runs.
8. **Idempotency.** A run request carries a caller `request_id`. Starting the same `request_id` for a run whose intent exists returns that run's current state and does not start another child.
9. **Layout additions** (additive, no format bump): `content/evidence/captures/`, `content/evidence/reports/`, `.functor/runs/` (staging, per-run locks, not repository state). Old binaries ignore all of them; `V0016` does not apply because they are outside `artifacts/` and `relationships/`.
10. **Timestamps** come from the injected `Clock`: `started` is recorded in the intent, `finished` in the result.

## Consequences

- No multi-entity transaction is needed: one commit-point record plus idempotent repairs.
- A crash at any step leaves either nothing, harmless orphans, an interrupted run, or a complete run.
- Many small artifacts per run (about 2 + inputs + outputs). The load cost is covered by the existing O(N) load budgets and is measured in P1.6.
- Pre-Phase 1 binaries can still `artifact update` an evidence artifact. This is a documented limitation, not an enforced one.
- Phase 2 can bind these captures to immutable versions without rewriting them.

## RFC Impact

Implements C-0001-5 and C-0007-4/5 for local runs. Does not implement RFC-0016 history; `revision` is not used as reproducibility evidence.
