# ADR-0010: async-trait; dynamic dispatch allowed

## Status

Accepted. Supersedes [ADR-0002](ADR-0002-async-core-traits.md).

## Context

ADR-0002 used native `async fn` in traits with static dispatch only, which is not dyn-compatible. The project owner decided that core traits must also be usable as trait objects, so a future projection or plugin host can hold `Box<dyn ...>` services.

## Decision

1. Core service traits in `functor-core` use the `async-trait` crate (`#[async_trait]`). They are dyn-compatible, and `Box<dyn Trait>` and `Arc<dyn Trait>` are allowed.
2. Futures returned through `async-trait` are `Send` by default, which resolves the `Send` question left open by ADR-0002. Implementations must therefore be `Send + Sync`.
3. Traits must stay object-safe: **no associated types, no generic methods, no return-position `impl Trait`**. Where an associated type was used, return a concrete type or a boxed trait object instead (for example `EventSource::subscribe` returns `Box<dyn EventSubscription>`, and `RepositoryManager` returns a concrete handle).
4. Static generics remain the recommended default inside services. The composition root (`functor-cli`) may use generics or trait objects, whichever is simpler.
5. Runtime rules from ADR-0002 are kept: only `functor-repository`, `functor-store`, `functor-events` and `functor-cli` may depend on `tokio`; `functor-model`, `functor-core`, `functor-query` and `functor-validation` must not name a runtime. Blocking filesystem calls use `tokio::fs` or `spawn_blocking`.
6. `async-trait` is approved for `functor-core` (a lower-layer dependency, which requires an ADR per [../dependency-policy.md](../architecture/dependency-policy.md)) and for the crates that implement those traits.

## Consequences

- Each async trait call allocates a boxed future. This is negligible for a local filesystem CLI.
- Core trait signatures must be reviewed for object safety; a compile-time test asserts that every core trait can be used as `dyn Trait` (`fn _assert(_: &dyn ArtifactRepository) {}`).
- If native async trait objects stabilize, the attribute can be removed without changing callers.

## RFC Impact

None. RFC-0002 does not prescribe sync or async or dispatch style.
