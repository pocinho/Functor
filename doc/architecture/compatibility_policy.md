# Compatibility Policy

This policy defines the compatibility promises Functor makes before external extensions depend on its persisted formats or integration contracts. Internal refactoring is allowed when these observable guarantees remain unchanged.

## Persisted Settings And Themes

- The persisted theme/settings schema is version `1`.
- The schema version is read before mapping values into application settings.
- Documents without a schema version, malformed versions, and versions newer than the supported version are rejected with an explicit load error.
- Older supported versions are migrated by `ThemeSchemaMigration`; migrations must be deterministic and covered by tests.
- New optional fields should have backward-compatible defaults. Changing the meaning or type of an existing field requires a new schema version and migration.
- A newer writer must not silently overwrite data it cannot read or preserve.

The schema version is independent of the application, assembly, or file format build number.

## Plugins

- The current plugin context is an internal, manually composed contract for trusted Functor-owned code.
- There is no supported third-party package format, dynamic discovery protocol, marketplace contract, sandbox, or stable external plugin API yet.
- Capability names and behavior are not compatibility promises until a versioned plugin manifest and negotiation step are implemented.
- In-process plugins are trusted code, not a security sandbox. Capability checks constrain the Functor API surface but cannot restrict arbitrary process access.
- Future plugin API changes must distinguish incompatible activation failure from runtime execution failure and must negotiate compatibility before activation.

## LSP Support

- The current LSP layer uses an internal transport-neutral request/response contract.
- It supports language start/stop, document publication, and diagnostics with document revisions, cancellation, bounded requests, and stale-result rejection.
- No external LSP server compatibility matrix or protocol-version promise is made by this contract. Wire-level LSP negotiation belongs in a future transport adapter.
- Protocol changes that alter request meaning, response interpretation, position semantics, or failure behavior require an explicit adapter/version review.

## Agent Capabilities

- Agent commands are an internal application adapter contract, not a stable remote API.
- Requests require correlation IDs, authorization, declared capabilities, cancellation, and an attributed completion audit event.
- Unsupported commands and unavailable capabilities are rejected explicitly; they are not silently downgraded.
- Agent capability names and argument maps may change until a versioned external protocol and compatibility negotiation are published.
- A future remote agent protocol must preserve authorization, cancellation, audit attribution, and failure classification across the wire.

## Change Review

A compatibility-affecting change must update the relevant version or policy section, add historical or boundary tests, and document migration or rejection behavior. A build or assembly version change alone does not authorize a breaking format or contract change.
