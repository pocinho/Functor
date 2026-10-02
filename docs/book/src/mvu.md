# MVU Core

Functor’s MVU engine is inspired by Elm and F# MVU, adapted for Rust’s ownership model.

Key principles:

- **Immutable Model** — cloned or diffed efficiently.
- **Pure Update** — no side effects.
- **Commands** — async tasks triggered by messages.
- **Subscriptions** — external event streams (FS changes, agent responses).

The MVU engine is the heart of Functors and ensures deterministic behavior.
