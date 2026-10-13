# Pull Request: <title>

## Summary
Provide a clear summary of what this PR does and why it is needed.
Reference the milestone ([docs/planning/cli-mvp-roadmap.md](../docs/planning/cli-mvp-roadmap.md)) or feature branch if applicable.

Example:
- Implements M7: `functor-validation` and `functor validate`
- Adds the acceptance test and the `examples/getting-started/` fixture
- Updates documentation for the milestone

---

## Source & Target Branches
- **Source:** `contributor/<feature>` or `feature/<milestone>`
- **Target:** `develop`

All PRs must target `develop`.  
Direct merges into `main` are not allowed.

---

## Changes Included
List the main changes introduced by this PR:

- New crates or modules added
- Refactors performed
- Documentation updated
- Tests added or modified

---

## Documentation
Check all that apply:

- [ ] Updated a contract document in `docs/design/`, `docs/cli/` (including `docs/cli/schemas/`) or `docs/architecture/`
- [ ] Updated `docs/planning/rfc-conformance.md` (statuses and test names)
- [ ] Added or updated an ADR in `docs/adr/`
- [ ] Updated the milestone status in `docs/planning/cli-mvp-roadmap.md`
- [ ] No documentation changes required

---

## Testing
Describe how this change was tested. Give each command with its result (passed or failed), and say what was not run.

- [ ] `pwsh scripts/ci.ps1` (fmt, clippy, dependency edges, RFC conformance, tests, markdown links)
- [ ] `cargo deny check`
- [ ] `cargo doc --workspace --no-deps` with `RUSTDOCFLAGS=-D warnings`
- [ ] Golden snapshots changed deliberately and the diff reviewed (`INSTA_UPDATE` is never set in CI)
- [ ] Manual checks performed (describe)

Windows is the only blocking platform; Linux and macOS results are informational ([docs/quality/nonfunctional-requirements.md](../docs/quality/nonfunctional-requirements.md)).

---

## Merge Strategy
This PR must be merged using:

- **Squash and merge** (into `develop`)

Release PRs from `develop` → `main` use a normal merge commit.

---

## Additional Notes
Add any extra context, follow‑up tasks, or known limitations.
