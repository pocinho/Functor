# Pull Request: <title>

## Summary
Provide a clear summary of what this PR does and why it is needed.
Reference the milestone or feature branch if applicable.

Example:
- Implements MVU core (`Model`, `Message`, `Update`)
- Adds initial WGPU rendering pipeline
- Updates documentation for Phase 1

---

## Source & Target Branches
- **Source:** `pp/<feature>` or `feature/<milestone>`
- **Target:** `develop`

All PRs must target `develop`.  
Direct merges into `main` are not allowed.

---

## Changes Included
List the main changes introduced by this PR:

- New modules added
- Refactors performed
- Documentation updated
- Tests added or modified

---

## Documentation
Check all that apply:

- [ ] Updated the developer book (`books/dev_book/`)
- [ ] Updated the user book (`books/user_book/`)
- [ ] Updated API documentation if applicable
- [ ] Added or updated architecture notes
- [ ] No documentation changes required

---

## Testing
Describe how this change was tested:

- Unit tests added
- Manual testing performed
- Rendering validated
- MVU update loop verified

---

## Merge Strategy
This PR must be merged using:

- **Squash and merge** (into `develop`)

Release PRs from `develop` → `main` use a normal merge commit.

---

## Additional Notes
Add any extra context, follow‑up tasks, or known limitations.
