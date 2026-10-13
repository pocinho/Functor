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
- Add `functor artifact delete`
- Add a `--format` option to `functor query`
- Add a new relationship type or validation rule
- Add partial loading for large repositories

---

## Motivation
Explain *why* this feature is important.

Consider:
- What problem does it solve?
- How does it improve the user experience?
- How does it fit into Functor's architecture, the RFCs or the roadmap (`docs/planning/product-roadmap.md`)? Name the RFC if it is about something the MVP deferred (see `docs/planning/rfc-conformance.md`).

---

## Proposed Design
Describe how you imagine the feature working.

You may include:
- command line syntax and output (text and JSON)
- API sketches (the service traits live in `functor-core`; see `docs/architecture/traits-and-interfaces.md`)
- file format or event changes (these are frozen contracts: see `docs/planning/mvp-scope.md`)
- validation rules or exit codes affected

Example:

```bash
functor artifact delete rfc-0000 --force
```

---

## Alternatives Considered
List any alternative approaches you considered and why they were rejected.

---

## Impact
Describe the expected impact of this feature.

Consider:

- Contracts: file format, JSON schemas, rule codes, exit codes, event envelope
- Performance (the budgets are in `docs/quality/nonfunctional-requirements.md`)
- Architectural changes (crate dependency edges, an ADR)
- Documentation updates
- Testing requirements
- Backward compatibility

---

## Additional Context
Add any other relevant information:

- Related issues
- Links to design documents or RFCs
- References to other projects
- Diagrams or sketches

---

## Checklist

- [ ] I have searched existing issues
- [ ] I believe this feature fits Functor's RFCs and roadmap
- [ ] I included motivation and design details
- [ ] I considered the frozen contracts it may change
- [ ] I included alternatives (if applicable)
