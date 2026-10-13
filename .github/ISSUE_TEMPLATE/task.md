---
name: Task / Milestone Item
about: Track a planned piece of work within a milestone
title: "[Task] <short description>"
labels: task
assignees: pocinho
---

## Goal
Describe the purpose of this task and what it aims to achieve.

Example:
- Add `functor artifact delete` with its validation and events
- Add partial loading for `artifact show`
- Implement the next phase of the product roadmap
- Write an ADR for a design decision

---

## Milestone
Specify which milestone or phase this task belongs to.

Example:
- CLI MVP follow-up (see `docs/planning/cli-mvp-roadmap.md`)
- Phase 1: Execution and Provenance (see `docs/planning/product-roadmap.md`)
- Phase 2: Versioning and History
- Phase 3: Capabilities and Extensions

---

## Description
Provide details about the work involved.

You may include:
- crates and modules affected
- API sketches
- architectural notes (and the ADR to write or update)
- references to RFCs and to the documents that define the contract
- references to other issues

---

## Acceptance Criteria
Define what must be true for this task to be considered complete.

Examples:
- The behavior is implemented as specified in the named document
- `pwsh scripts/ci.ps1`, `cargo deny check` and `cargo doc` pass
- Contract documents and `docs/planning/rfc-conformance.md` are updated, and every named test exists
- Tests added for the new behavior and for its failure cases

---

## Dependencies
List any tasks or issues that must be completed first.

Example:
- Depends on #12 (event envelope change)
- Depends on #8 (ADR for the new format version)

---

## Documentation
Indicate required documentation updates.

- [ ] Update a contract document in `docs/design/`, `docs/cli/` (including `docs/cli/schemas/`) or `docs/architecture/`
- [ ] Update `docs/planning/rfc-conformance.md`
- [ ] Add or update an ADR in `docs/adr/`
- [ ] Update the status in `docs/planning/cli-mvp-roadmap.md` or `docs/planning/product-roadmap.md`
- [ ] No documentation required

---

## Testing
Describe the testing requirements.

- [ ] Unit tests
- [ ] Store conformance suite (if a store changes)
- [ ] End-to-end CLI tests with golden snapshots and JSON schema checks
- [ ] Acceptance test or library workflow test updated
- [ ] Manual testing (describe)
- [ ] No tests required

---

## Additional Notes
Add any extra context or follow-up tasks.
