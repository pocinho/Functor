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
- Write architecture chapter for the Functor Developer Book

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
- Documentation updated in the appropriate Functor book
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

- [ ] Update developer book (`books/dev_book/`)
- [ ] Update user book (`books/user_book/`)
- [ ] Update API documentation if applicable
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
