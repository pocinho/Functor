# ADR-0002: Async core traits with static dispatch

## Status

Accepted

## Context

`api-design-guidelines.md` said I/O-facing traits should be asynchronous, while `traits-and-interfaces.md` showed synchronous traits. Native `async fn` in traits (stable since Rust 1.75, available in edition 2024) is **not dyn-compatible**. Future projections (web, MCP, desktop) and event watching benefit from an async API, while the CLI MVP is a local-filesystem tool.

## Decision

1. Core service traits in `functor-core` are asynchronous, using native `async fn` in traits.
2. Services are composed through **generics (static dispatch)** at the composition root, which is `functor-cli`. Traits are not used as `dyn` objects in the MVP.
3. If a later projection needs dynamic dispatch, introduce `trait_variant` or `async-trait` on the specific trait through a new ADR. Do not add it preemptively.
4. Futures returned by core traits should be `Send` so projections can run on a multi-threaded runtime. Use `trait_variant::make(Send)` or explicit `+ Send` bounds in signatures when the first multi-threaded projection appears; until then, `functor-cli` uses a current-thread runtime.
5. Only `functor-store`, `functor-events` and `functor-cli` may depend on an async runtime (`tokio`). `functor-model`, `functor-core`, `functor-query` and `functor-validation` depend only on `core::future` and must not name a runtime.
6. Blocking filesystem calls inside `functor-store` use `tokio::fs` or `spawn_blocking`, never a blocking call on the async executor thread.

## Consequences

- No `Box<dyn ArtifactRepository>`. Tests use generic parameters or test doubles implementing the traits.
- Pure computation (query traversal, validation rules over loaded data) is plain synchronous code over in-memory data. Only the I/O boundary is async, which keeps most logic trivially testable.
- Slightly higher compile-time cost and more generic parameters in the CLI wiring.

## RFC Impact

None. RFC-0002 does not prescribe sync or async. This supports RFC-0002's requirement that projections be replaceable.
