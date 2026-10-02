# Contributing to Functors

Functors is in early development. This document defines the initial workflow and
contribution guidelines. It will evolve as the project grows.

## Branching Strategy

- **main**  
  Stable branch. Always compiles. Tagged releases.

- **develop**  
  Integration branch for upcoming features.

- **feature/***  
  Each milestone or major feature is developed in its own feature branch.  
  Examples:  
  - `feature/mvu-core`  
  - `feature/rendering-engine`  
  - `feature/text-buffer`

Feature branches merge into `develop` via pull requests.

## Commit Messages

Use Conventional Commits:

- `feat:` new feature  
- `fix:` bug fix  
- `docs:` documentation  
- `refactor:` internal changes  
- `test:` tests  
- `chore:` maintenance

Examples:
- `feat(mvu): add Model and Message definitions`
- `docs: add MVU architecture chapter`

## Code Style

- Rust 2024 edition  
- Clippy must pass  
- `rustfmt` must pass  
- Prefer small, composable modules  
- MVU code must remain pure and deterministic

## Documentation

All major features must update:

- `docs/book/` (architecture, design, roadmap)  
- `docs/api/` (crate-level documentation)

## Testing

Phase 1: basic unit tests for MVU core.  
More extensive testing will be added in later phases.

---

This file will expand as Functors grows.
