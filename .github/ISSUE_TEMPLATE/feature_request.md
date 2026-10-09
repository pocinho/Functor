---
name: Feature Request
about: Propose a new feature or enhancement for Functor
title: "[Feature] <short description>"
labels: enhancement
assignees: pocinho
---

## Summary
Describe the feature you would like to see added to Functor.

Example:
- Add MVU command batching
- Implement GPU-driven text rendering
- Introduce notebook cell dependency graph
- Add agent protocol message routing

---

## Motivation
Explain *why* this feature is important.

Consider:
- What problem does it solve?
- How does it improve the user experience?
- How does it fit into Functor’ architecture or roadmap?

---

## Proposed Design
Describe how you imagine the feature working.

You may include:
- API sketches
- module structure
- MVU model/message/update changes
- rendering pipeline changes
- notebook runtime behavior
- agent protocol interactions

Example:

```rust
pub enum Message {
    LoadFile(PathBuf),
    RenderFrame,
    // ...
}
```

---

## Alternatives Considered
List any alternative approaches you considered and why they were rejected.

---

## Impact
Describe the expected impact of this feature.

Consider:

- Performance implications
- Architectural changes
- Documentation updates
- Testing requirements
- Backward compatibility

---

## Additional Context
Add any other relevant information:

- Related issues
- Links to design documents
- References to other projects
- Diagrams or sketches

---

## Checklist

- [ ] I have searched existing issues
- [ ] I believe this feature fits Functor’ roadmap
- [ ] I included motivation and design details
- [ ] I included alternatives (if applicable)
