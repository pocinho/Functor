# RFC Conformance Matrix

Status: Accepted (kept current by every pull request; checked by `scripts/check-conformance.ps1`)
Related docs: [mvp-scope.md](mvp-scope.md), [testing-strategy.md](../quality/testing-strategy.md), [adr/README.md](../adr/README.md)

## Purpose

`docs/README.md` says that when implementation guidance conflicts with an RFC, the RFC wins. This matrix makes that rule operational: it lists the normative requirements the MVP touches, states for each whether it is implemented, implemented as a subset, deferred, or deliberately deviated from, and names where it lives and the test that proves it.

## Status Values

| Status | Meaning |
|--------|---------|
| Implemented | Satisfied fully by the MVP |
| Subset | Partially satisfied; the gap is stated |
| Deferred | Intentionally outside the MVP |
| Deviation | The MVP does something different from the RFC; an ADR or note explains why |

## Rules for Maintaining This Table

- Every pull request that changes behavior updates the affected rows.
- "Implemented" and "Subset" rows must name a test, and every named test must exist: `scripts/check-conformance.ps1` (part of `scripts/ci.ps1` and CI) fails on an unknown status, a row without a test and a reference that does not resolve to a function, a wildcard match, a module or a test file.
- A "Deviation" requires an ADR. RFCs are never edited to match the implementation; a needed RFC change is proposed as a new RFC revision.
- Keep requirement text short and cite the RFC section.

## Matrix

Test column notation: each reference is `prefix:name` in backticks. `model:` unit and property tests in `functor-model`; `repo:` tests in `functor-repository`; `store:` tests in `functor-store`; `conf:` the store conformance suite (`functor-testkit`); `query:`, `val:`, `events:`, `rt:` (`functor-runtime`) crate tests; `cli:` end-to-end tests in `functor-cli`; `acc:` the acceptance and library-workflow tests (`src/app/cli/tests/acceptance.rs` and `library.rs`); `script:` a check in `scripts/` that is not a cargo test. A name is a test function, optionally qualified by its module or test file (`cli:events::name`); `name_*` is a wildcard, `module::*` the whole module and a bare `cli:validate` the whole test file.

### RFC-0001 Core Semantic Model

| ID | Requirement | Status | Implementation | Test |
|----|-------------|--------|----------------|------|
| C-0001-1 | Every artifact has identity, metadata, content and relationships | Implemented | [domain-model.md](../design/domain-model.md) | `model:create_starts_at_revision_one`, `store:entities_are_stored_as_canonical_files_named_by_id`, `conf:create_get_resolve_and_list` |
| C-0001-2 | Identity is immutable | Implemented | `ArtifactPatch` has no id field; update never changes `id` | `conf:update_increments_revision_and_preserves_identity` |
| C-0001-3 | Relationships are first-class entities | Implemented | Own type, own id, own file | `conf:relationship_get_and_list_order`, `cli:relation::add_stores_a_canonical_file_named_by_id` |
| C-0001-4 | Validation results are artifacts themselves | Implemented | Opt-in immutable report artifacts through the library and `validate --save-report` ([ADR-0015](../adr/ADR-0015-validation-report-persistence-and-events.md)); the ordinary validation path remains read-only and byte-compatible | `cli:validate::validation_never_changes_the_repository_state`, `val:engine::validation_does_not_modify_the_repository`, `rt:orchestrator::output_validation_persists_a_report_per_output_without_changing_the_run`, `rt:orchestrator::reports_are_never_validated_in_turn_and_validate_alone_stays_read_only`, `cli:run::saved_validation_is_opt_in_and_failure_preserves_the_computed_report`, `cli:run::saved_reports_keep_validation_outcomes_and_subjects_separate_from_persistence` |
| C-0001-5 | Execution is a reproducible operation producing artifacts | Subset | Local runs through the library and CLI: captured inputs, durable intent, outputs as artifacts, replay from captures ([ADR-0012](../adr/ADR-0012-execution-provider-and-safety.md), [ADR-0013](../adr/ADR-0013-run-lifecycle-and-evidence-persistence.md)). No sandbox or network enforcement, a Phase 3 and Phase 6 topic | `rt:end_to_end::replaying_a_captured_input_reproduces_the_output_in_a_new_run`, `rt:end_to_end::a_real_run_is_recorded_durably_and_survives_a_restart`, `rt:orchestrator::every_child_outcome_is_recorded_not_raised`, `cli:run::execution_show_and_list_cross_read_with_the_library_after_restart`, `cli:run::each_child_outcome_has_a_result_and_the_documented_exit_code` |
| C-0001-6 | Projections do not own the model | Implemented | CLI is a thin projection ([cli-spec.md](../cli/cli-spec.md)) | `acc:the_success_path_and_the_negative_variant_run_through_the_libraries` |

### RFC-0002 API-First Architecture

| ID | Requirement | Status | Implementation | Test |
|----|-------------|--------|----------------|------|
| C-0002-1 | Every UI capability is expressible through the core API | Implemented | `functor-core` traits; CLI calls only traits | `acc:the_success_path_and_the_negative_variant_run_through_the_libraries`, `script:check-deps.ps1` |
| C-0002-2 | All projections operate on the same semantic model | Implemented | Single model crate | `acc:a_repository_written_by_the_libraries_is_read_by_the_cli`, `acc:a_repository_written_by_the_cli_is_read_by_the_libraries` |
| C-0002-3 | Projections are replaceable without core changes | Implemented | Layering in [crate-dependencies.md](../architecture/crate-dependencies.md) | `script:check-deps.ps1` |
| C-0002-4 | MCP is an integration layer, not core | Deferred | | |

### RFC-0003 Artifact Model

| ID | Requirement | Status | Implementation | Test |
|----|-------------|--------|----------------|------|
| C-0003-1 | Artifact provides Identity, Type, Metadata, Version, Content | Subset | Version is the integer `revision`; no history. [artifact-format.md](../design/artifact-format.md) | `conf:update_increments_revision_and_preserves_identity`, `model:patch_increments_revision_and_reports_changes` |
| C-0003-2 | Identity is stable and independent of storage location, filename, UI and protocol | Implemented | Opaque id inside the file; filename is derived from id, never the reverse | `conf:slug_rename_releases_the_old_slug_and_keeps_relationships`, `store:entities_are_stored_as_canonical_files_named_by_id` |
| C-0003-3 | Types are extensible | Implemented | Open `ArtifactType` set | `model:unknown_artifact_types_are_accepted` |
| C-0003-4 | Metadata is extensible | Implemented | `[metadata]` table | `store:metadata_of_every_type_round_trips` |
| C-0003-5 | Content format is not mandated | Implemented | `content.format` is free text | `store:inline_content_with_newlines_round_trips`, `store:content_can_be_replaced_and_cleared` |
| C-0003-6 | Artifact-centric API: create, get, update, delete, relate, validate, query | Subset | All except delete ([traits-and-interfaces.md](../architecture/traits-and-interfaces.md)) | `conf:create_get_resolve_and_list`, `conf:update_increments_revision_and_preserves_identity`, `conf:relationship_rules`, `cli:artifact`, `cli:relation`, `cli:query`, `cli:validate` |
| C-0003-7 | Composition (artifacts containing artifacts) | Deferred | Expressible with relationships only | |
| C-0003-8 | Execution capability by artifact type | Deferred | | |

### RFC-0004 Relationship Model

| ID | Requirement | Status | Implementation | Test |
|----|-------------|--------|----------------|------|
| C-0004-1 | Relationships are directional | Implemented | source and target | `model:relationship_direction_and_self_reference` |
| C-0004-2 | Core types: references, depends_on, derived_from, extends, supports, contradicts | Implemented | All six built in; `supports`, `contradicts` (and `validates`) became built-in in Phase 1 ([ADR-0014](../adr/ADR-0014-evidence-relationships.md), amending [ADR-0007](../adr/ADR-0007-relationship-types.md)). The `V0023` conflict warning is implemented (P1.4) | `model:built_in_names_parse_to_built_ins`, `model:evidence_types_are_built_in_but_may_form_cycles`, `model:evidence_direction_and_self_reference`, `model:custom_types_are_flagged_and_validated`, `store:self_reference_of_acyclic_and_evidence_types_is_reported_not_built`, `val:rules::v0012_does_not_fire_for_evidence_types`, `val:rules::v0023_a_source_that_supports_and_contradicts_the_same_target`, `rt:orchestrator::evidence_assertions_have_a_direction_and_the_rules_see_conflicts_and_gaps` |
| C-0004-3 | Relationships should have stable identifiers | Implemented | `rel_<ULID>` | `conf:relationship_get_and_list_order`, `cli:relation::add_stores_a_canonical_file_named_by_id` |
| C-0004-4 | Metadata must not alter semantic meaning | Implemented | Opaque metadata map | `store:rendered_relationships_parse_back` |
| C-0004-5 | Validate existence of referenced artifacts and detect cycles | Implemented | `V0006`, `V0007`, `V0009`, `V0011` ([validation-rules.md](../design/validation-rules.md)) | `val:rules::v0006_dangling_source`, `val:rules::v0007_dangling_target`, `val:rules::v0009_self_reference`, `val:rules::v0011_*` |
| C-0004-6 | API: create, remove, list, find_related, traverse | Subset | No remove | `conf:relationships`, `query:traverse::*`, `cli:query`, `cli:relation` |
| C-0004-7 | Reverse relationships via indexing or query | Implemented | Inbound traversal ([query-model-mvp.md](../design/query-model-mvp.md)) | `query:properties::inbound_of_the_outbound_neighbours_includes_the_start`, `cli:query::dependents_follow_edges_backwards` |

### RFC-0005 Identity and Addressing

| ID | Requirement | Status | Implementation | Test |
|----|-------------|--------|----------------|------|
| C-0005-1 | Identity immutable through content, metadata and relationship changes | Implemented | | `conf:update_increments_revision_and_preserves_identity` |
| C-0005-2 | Identifiers do not encode paths, keys or URLs | Implemented | `art_<ULID>` | `model:id_text_round_trips`, `model:id_rejects_bad_text` |
| C-0005-3 | Identifiers are opaque to consumers | Implemented | No timestamp accessor; slug is a separate alias ([ADR-0001](../adr/ADR-0001-two-part-identity.md)) | `model:id_kinds_do_not_parse_each_other`, `model:malformed_id_is_not_retried_as_slug`; the absence of accessors is an API-review rule |
| C-0005-4 | Globally unique identifiers | Subset | ULID: probabilistically unique; uniqueness within a repository is additionally checked (`V0003`) | `val:rules::v0003_duplicate_artifact_id`, `val:engine::v0003_duplicate_artifact_id` |
| C-0005-5 | Canonical reference `functor:<entity-type>:<id>` (SHOULD) | Implemented | Accepted on input, emitted as `ref` in JSON ([identity-and-addressing.md](../design/identity-and-addressing.md)) | `model:canonical_reference_round_trips`, `cli:relation::add_accepts_slugs_ids_canonical_references_and_the_kebab_alias`, `cli:query::start_artifacts_resolve_by_slug_id_and_canonical_reference` |
| C-0005-6 | Retrieval by identifier: get, resolve, exists | Implemented | `get`, `resolve`, `lookup` | `conf:create_get_resolve_and_list` |
| C-0005-7 | Cross-repository and federated identifiers | Deferred | | |

### RFC-0006 Query Model

See the coverage table in [query-model-mvp.md](../design/query-model-mvp.md).

| ID | Requirement | Status | Implementation | Test |
|----|-------------|--------|----------------|------|
| C-0006-1 | Lookup by identity | Implemented | `lookup` | `query:engine::memory_and_file_stores_answer_identically` |
| C-0006-2 | Search by metadata and content | Subset | Metadata, type, tag, text on slug, title and description; no content search | `query:search::*`, `cli:query::artifact_criteria_combine_with_and` |
| C-0006-3 | Traverse relationships, inbound, outbound, bidirectional, depth limit | Implemented | `traverse` | `query:traverse::*`, `query:properties::traversal_matches_a_brute_force_oracle` |
| C-0006-4 | Storage-independent semantics | Implemented | Queries run over snapshots | `query:engine::memory_and_file_stores_answer_identically` |
| C-0006-5 | Results immutable during execution | Implemented | Single snapshot per query | `query:engine::every_engine_call_reads_exactly_one_snapshot`, `cli:query::queries_never_change_the_repository` |
| C-0006-6 | Filter and project | Subset | Filter fields in `SearchQuery`; projection is a rendering concern of the CLI (fixed columns, full JSON) | `query:search::*`, `cli:query::artifacts_in_json_matches_the_schema` |
| C-0006-7 | Query language, ranking, federation | Deferred | | |

### RFC-0007 Validation and Execution

| ID | Requirement | Status | Implementation | Test |
|----|-------------|--------|----------------|------|
| C-0007-1 | Validation outcomes explicit: Passed, Failed, Warning, Unknown | Implemented | `Outcome` | `model:outcome_derivation_table`, `val:evaluate::the_outcome_follows_the_worst_finding`, `val:evaluate::an_unreadable_path_makes_the_outcome_unknown_unless_there_is_an_error`, `cli:exit::validation_outcomes_map_to_the_documented_exit_codes` |
| C-0007-2 | Validation of artifacts and relationships | Implemented | Rule catalog `V0001` to `V0023` | `val:rules::v0001_*` to `val:rules::v0019_*`, `val:rules::v0020_*`, `val:rules::v0021_*`, `val:rules::v0022_*`, `val:rules::v0023_*`, `val:engine::v0001_*` to `val:engine::v0018_*` (real files), `cli:validate` |
| C-0007-3 | Validation outputs representable as artifacts | Implemented | See C-0001-4; the library and CLI store reports as immutable artifacts | `store:evidence_is_ordinary_artifacts_and_relationships_that_trace_both_ways`, `rt:orchestrator::a_caller_computed_report_is_recorded_with_its_event`, `cli:run::saved_validation_is_opt_in_and_failure_preserves_the_computed_report` |
| C-0007-4 | Execution as evidence; reproducible executions | Subset | See C-0001-5 | `rt:end_to_end::replaying_a_captured_input_reproduces_the_output_in_a_new_run`, `rt:orchestrator::a_successful_run_leaves_traceable_evidence_and_two_events` |
| C-0007-5 | Traceability between inputs, executions, outputs, validations | Implemented | Run evidence is persisted and traceable both ways as artifacts and relationships ([ADR-0014](../adr/ADR-0014-evidence-relationships.md), [ADR-0016](../adr/ADR-0016-evidence-files-and-repair.md)), orchestrated with events and validation rules `V0020` to `V0023`, and cross-readable through CLI and libraries | `store:evidence_is_ordinary_artifacts_and_relationships_that_trace_both_ways`, `store:a_failure_at_every_write_never_reports_a_false_success_and_always_recovers`, `store:evidence_survives_a_restart_and_a_moved_repository`, `rt:orchestrator::evidence_is_navigable_from_a_conclusion_to_the_inputs_and_back`, `rt:orchestrator::interrupted_and_orphaned_work_is_visible_to_validation`, `cli:run::execution_show_and_list_cross_read_with_the_library_after_restart` |

### RFC-0008 Storage and Persistence

| ID | Requirement | Status | Implementation | Test |
|----|-------------|--------|----------------|------|
| C-0008-1 | Semantic model independent of any storage technology | Implemented | Model and core have no storage knowledge; two stores | `conf:*` on both stores |
| C-0008-2 | Durable identity across storage engines | Implemented | Id is in the entity, not the location | `acc:a_moved_repository_keeps_every_identity` |
| C-0008-3 | Local-first, offline | Implemented | Filesystem only | `script:check-deps.ps1` (no network-capable crate in the shipped dependency tree) |
| C-0008-4 | Preserve historical states when practical | Deferred | `revision` only | |
| C-0008-5 | Indexes must not affect semantics | Implemented | In-memory indexes rebuilt per snapshot | `query:engine::memory_and_file_stores_answer_identically` |
| C-0008-6 | Portability, no proprietary representation | Implemented | TOML and JSON Lines | `store:crlf_checkouts_load_and_keep_their_line_endings_when_updated` |

### RFC-0011 Projection Model

| ID | Requirement | Status | Implementation | Test |
|----|-------------|--------|----------------|------|
| C-0011-1 | CLI is a projection over the common API | Implemented | [cli-spec.md](../cli/cli-spec.md) | `acc:the_success_path_and_the_negative_variant_run_through_the_libraries` |
| C-0011-2 | Artifact identity unchanged across projections | Implemented | | `acc:a_repository_written_by_the_libraries_is_read_by_the_cli`, `acc:a_repository_written_by_the_cli_is_read_by_the_libraries` |
| C-0011-3 | Other projections | Deferred | | |

### RFC-0015 Repository Model

| ID | Requirement | Status | Implementation | Test |
|----|-------------|--------|----------------|------|
| C-0015-1 | Repository MUST have a stable identifier | Implemented | `repo_<ULID>` in `repository.toml` ([ADR-0009](../adr/ADR-0009-functor-repository-crate.md), `functor-repository`) | `repo:init_creates_the_documented_layout`, `repo:the_repository_id_is_stable_across_opens` |
| C-0015-2 | Repository metadata is informational and never alters semantics | Implemented | | `acc:repository_metadata_never_changes_what_the_repository_means` |
| C-0015-3 | Create, open, query, validate repository operations | Implemented | `init`, discovery, `query`, `validate` | `cli:init`, `cli:artifact::the_repository_is_discovered_from_a_subdirectory`, `cli:query`, `cli:validate` |
| C-0015-4 | Synchronize, export, import | Deferred | | |
| C-0015-5 | Repository not tied to a storage technology | Implemented | `RepositoryManager` trait (`functor-repository`) and the store traits | `repo:init_twice_is_a_conflict_and_changes_nothing`, `repo:the_manager_uses_the_injected_clock_and_ids`, `conf:*` |

### RFC-0017 Event Model

| ID | Requirement | Status | Implementation | Test |
|----|-------------|--------|----------------|------|
| C-0017-1 | Every event contains identifier, type, timestamp, origin, subject, metadata | Implemented | Envelope in [event-model-mvp.md](../design/event-model-mvp.md) | `events:wire::encoding_uses_the_documented_key_order_and_shape`, `events:wire::events_round_trip`, `cli:events::the_event_log_after_a_typical_session_is_stable` |
| C-0017-2 | Hierarchical event type names | Implemented | `artifact.created` and so on | `events:wire::encoding_uses_the_documented_key_order_and_shape`, `cli:watch::type_filters_match_names_and_prefixes` |
| C-0017-3 | Event types: artifact, relationship, validation (and others) | Subset | Seven types: the four original types plus `run.started`, `run.completed` and `validation.recorded`; capability and extension events remain deferred | `cli:events::artifact_create_emits_artifact_created`, `cli:events::artifact_update_emits_artifact_updated_listing_the_changes`, `cli:events::relation_add_emits_relationship_created`, `cli:events::validate_always_emits_validation_completed_even_when_it_fails`, `cli:events::read_only_and_failed_commands_emit_nothing`, `cli:run::watch_observes_run_and_validation_events_without_writing` |
| C-0017-4 | Events are informational, not authoritative | Implemented | State never depends on the log | `cli:events::a_failed_append_warns_and_the_command_still_succeeds`, `cli:events::a_failed_append_warns_for_every_mutating_command_in_both_modes` |
| C-0017-5 | Do not mandate transport, serialization or synchronous processing | Implemented | Local JSON Lines is an implementation choice | `acc:the_success_path_and_the_negative_variant_run_through_the_libraries` (publishes and subscribes through the `EventPublisher` and `EventSource` traits) |
| C-0017-6 | Auditability | Subset | Append-only log, rotated at 10 MiB with 10 archives retained ([ADR-0008](../adr/ADR-0008-event-log-rotation.md)); history older than the retained window is lost; no signatures | `events:log::sequence_numbers_are_gap_free_across_the_current_log_and_every_archive`, `events:log::exactly_the_ten_newest_archives_are_retained_and_the_oldest_is_pruned`, `cli:events::concurrent_commands_never_duplicate_or_skip_a_sequence_number`, `cli:events::the_log_rotates_at_10_mib_and_numbering_continues` |
| C-0017-7 | Distributed ordering is not assumed | Implemented | `seq` is per repository only | `events:log::sequence_numbers_are_gap_free_across_the_current_log_and_every_archive`, `cli:events::concurrent_commands_never_duplicate_or_skip_a_sequence_number` |
| C-0017-8 | Subscriptions, replay, extension events | Deferred | `watch` is a simple tail | `events:follow::*`, `cli:watch` |

### Deferred RFCs

| RFC | Title | Status | Note |
|-----|-------|--------|------|
| 0009 | Federation and Synchronization | Deferred | Identity design leaves room for repository-qualified references |
| 0010 | Capability Model | Deferred | No registry; traits only |
| 0012 | Interoperability and Integration | Deferred | No MCP |
| 0013 | Security and Trust Model | Deferred | Local safety baseline in [nonfunctional-requirements.md](../quality/nonfunctional-requirements.md) |
| 0014 | Extension Model | Deferred | `metadata` and custom relationship and artifact types are the only extension points |
| 0016 | Versioning Model | Deferred | `revision` counter; versions published are immutable (RFC-0016 MUST) is **not** claimed |
| 0018 | Governance Model | Deferred | |

## Known Deviations (summary)

The former validation-report deviation (C-0001-4, C-0007-3) is closed by Phase 1 opt-in report persistence.

1. Artifact identity is two-part (id plus slug), a compatible extension of RFC-0005 ([ADR-0001](../adr/ADR-0001-two-part-identity.md)).
2. No deletion of artifacts or relationships (C-0003-6, C-0004-6).
3. Event history is bounded (C-0017-6): older events are pruned by rotation.
