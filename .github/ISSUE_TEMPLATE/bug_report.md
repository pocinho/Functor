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
- MVU update loop panics when receiving a specific message
- Rendering pipeline fails to initialize on Windows
- Notebook runtime crashes when loading a large file

---

## Environment
Please fill in all relevant details:

- OS: (Windows / macOS / Linux)
- Rust version: `rustc --version`
- Functor version or branch: (`develop`, `main`, or feature branch)
- GPU / graphics backend (if rendering-related)
- Terminal or shell (if CLI-related)

---

## Steps to Reproduce
Describe the exact steps needed to reproduce the issue.

1. Run `npm --prefix functor run tauri -- dev`
2. Open the notebook file
3. Trigger the rendering update
4. Observe the crash

Include code snippets if relevant:

```rust
// Minimal reproduction example
fn main() {
    // ...
}
```

---

## Expected Behavior
Describe what you expected to happen.

---

## Actual Behavior
Describe what actually happened.

Include logs, error messages, or stack traces:

```
thread 'main' panicked at ...
```

---

## Screenshots (optional)
If applicable, add screenshots or GIFs.

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
- [ ] I included logs or error messages
