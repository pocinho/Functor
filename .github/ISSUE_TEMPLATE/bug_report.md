---
name: Bug Report
about: Report a reproducible issue in Functor
title: "[Bug] <short description>"
labels: bug
assignees: pocinho
---

## Summary
Provide a clear and concise description of the bug.

Example:
- `functor validate` exits 0 on a repository with a dangling relationship
- `functor watch` prints the same event twice after a log rotation
- `functor artifact update` rewrites unrelated lines of an artifact file

---

## Environment
Please fill in all relevant details:

- OS: (Windows is the supported platform; Linux and macOS are best-effort and non-blocking)
- Rust version: `rustc --version`
- Functor version or branch: `functor --version` (or `develop`, `main`, or feature branch)
- Terminal or shell: (PowerShell, cmd, bash, ...)
- Output mode: (`text` or `--output json`)

---

## Steps to Reproduce
Describe the exact steps needed to reproduce the issue. A short command sequence on a new repository is ideal:

```bash
functor init --name "Repro"
functor artifact create rfc-0000 --type rfc --title "Vision"
functor relation add rfc-0000 depends_on rfc-0000
```

If the problem needs a particular repository state, attach or describe the files involved (`artifacts/`, `relationships/`, `events/`). Remove anything private first.

---

## Expected Behavior
Describe what you expected to happen. Cite the relevant document if there is one (for example the command in `docs/cli/cli-spec.md` or a rule in `docs/design/validation-rules.md`).

---

## Actual Behavior
Describe what actually happened, including the exit code (`echo $LASTEXITCODE`) and what was printed on stdout and stderr. Run with `-v` to include the cause chain of an error.

```
error: ...
```

---

## Additional Context
Add any other context about the problem:

- Related issues
- Recent changes
- Feature branch involved
- Possible cause (if known)

---

## Checklist

- [ ] I have searched existing issues
- [ ] I am using the latest version of develop
- [ ] I included reproduction steps
- [ ] I included the exit code and the error output
- [ ] I ran `functor validate` and included its output if the repository may be damaged
