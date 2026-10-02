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
- Implement MVU core (Model, Message, Update)
- Add initial WGPU rendering pipeline
- Create text buffer rope structure
- Write architecture chapter for mdBook

---

## Milestone
Specify which milestone or phase this task belongs to.

Example:
- Phase 1 — MVU + Rendering
- Phase 2 — Text Buffer + Layout
- Phase 3 — Notebook Runtime
- Phase 4 — Agent Protocol

---

## Description
Provide details about the work involved.

You may include:
- module structure
- API sketches
- architectural notes
- diagrams
- references to other issues

---

## Acceptance Criteria
Define what must be true for this task to be considered complete.

Examples:
- MVU core compiles and runs
- Rendering pipeline initializes without errors
- Text buffer supports basic editing operations
- Documentation updated in `docs/book/`
- Tests added for core functionality

---

## Dependencies
List any tasks or issues that must be completed first.

Example:
- Depends on #12 (GPU initialization)
- Depends on #8 (Model definition)

---

## Documentation
Indicate required documentation updates.

- [ ] Update mdBook (`docs/book/`)
- [ ] Update API docs (`docs/api/`)
- [ ] Add architecture notes
- [ ] No documentation required

---

## Testing
Describe the testing requirements.

- [ ] Unit tests
- [ ] Integration tests
- [ ] Manual testing
- [ ] Rendering validation
- [ ] No tests required

---

## Additional Notes
Add any extra context or follow-up tasks.
