# Contributing to Functor

Functor is in early development. This document defines the contribution workflow,
branching model, merge strategy, and coding standards. It will evolve as the
project grows.

---

# 1. Branching Model

Functor uses a disciplined branching strategy inspired by GitFlow, adapted for
solo and multi‑maintainer development.

## Protected Branches

### **main**
- Stable branch  
- Always compiles  
- Contains tagged releases  
- Direct pushes are **not allowed**  
- All changes must come through PRs

### **develop**
- Integration branch  
- Receives completed features  
- Direct pushes are **not allowed**  
- All changes must come through PRs

---

# 2. Feature Branches

Major features and milestones are developed in dedicated **feature/*** branches.

Examples:
- `feature/mvu-core`
- `feature/rendering-engine`
- `feature/text-buffer`
- `feature/notebook-model`
- `feature/agent-protocol`

Rules:
- Feature branches are long‑lived during a milestone  
- They represent the *clean* version of the milestone  
- They merge into `develop` via PR  
- They should not contain experimental or messy commits

Feature branches act as the “official” home of a milestone.

---

# 3. Personal Working Branches

All day‑to‑day development happens in personal branches derived from `develop`.

Examples:
- `contributor/mvu-core`
- `contributor/rendering-engine`
- `contributor/text-buffer`

Rules:
- You may commit freely  
- You may rebase, amend, or force‑push  
- These branches are not protected  
- These branches are used to open PRs into `feature/*` or directly into `develop`

Typical workflow:

develop → contributor/mvu-core → PR → feature/mvu-core → PR → develop

For solo development, this may simplify to:

develop → contributor/mvu-core → PR → develop

---

# 4. Pull Request Workflow

## PR Targets
- Personal working branches → `develop`  
- Feature branches → `develop`  
- `develop` → `main` (release PR)

## PR Requirements
- PR must compile  
- PR must include documentation updates if relevant  
- PR must follow the merge strategy below  
- PR must be approved (self‑approval allowed for solo maintainers)

---

# 5. Merge Strategy

### **Squash and merge** (for PRs into `develop`)
Use squash for:
- personal working branches → develop  
- feature branches → develop  

This keeps `develop` clean and milestone‑oriented.

### **Merge commit** (for develop → main)
Use a normal merge commit for:
- releases  
- version bumps  
- stable milestones  

This preserves the full milestone history.

### **Rebase**
Allowed **only** on personal working branches.  
Never rebase `main`, `develop`, or `feature/*` after pushing.

---

# 6. Commit Message Conventions

Functor uses **Conventional Commits**:

- `feat:` new feature  
- `fix:` bug fix  
- `docs:` documentation changes  
- `refactor:` internal changes  
- `test:` tests  
- `chore:` maintenance  

Examples:
- `feat(mvu): add Model and Message definitions`
- `docs: update MVU architecture chapter`
- `refactor(render): simplify GPU pipeline init`

---

# 7. Coding Standards

- Rust 2024 edition  
- `rustfmt` must pass  
- `clippy` must pass  
- Prefer small, composable modules  
- MVU code must remain pure and deterministic  
- Rendering code must avoid blocking operations  
- Platform code must isolate OS‑specific logic

---

# 8. Documentation Requirements

Update the owning chapter in `books/dev_book/` for engineering architecture,
decisions, requirements, milestones, or migration work.
Update `books/user_book/` when user-facing workflows or limitations change.
Build both mdBooks for documentation changes.

Documentation is part of the definition of done.

---

# 9. Testing

Phase 1:
- basic unit tests for MVU core  
- basic rendering initialization tests  

Later phases will introduce:
- integration tests  
- snapshot tests  
- WASM/WASI runtime tests  
- agent protocol tests  

---

# 10. Code of Conduct

All contributors must follow the project's Code of Conduct.

---

This document will expand as Functor grows.
