# Testing Strategy

## Purpose

Defines the testing approach for the Functor CLI MVP.

## Principles

- Test core semantics first.
- Favor deterministic tests.
- Validate RFC compliance.
- Keep tests independent.

## Unit Tests

Cover:

- Artifact models
- Identity rules
- Relationship rules
- Validation logic

## Integration Tests

Cover:

- Repository initialization
- Artifact creation
- Relationship creation
- Query workflows
- Validation workflows

## Repository Fixtures

Use small sample repositories as test fixtures.

## CLI Tests

Validate:

```bash
functor init
functor artifact create
functor relation add
functor query
functor validate
```

## Snapshot Tests

Use golden files for CLI output where practical.

## Acceptance Tests

The MVP is successful if the following workflow passes:

```bash
functor init
functor artifact create rfc-0000
functor artifact create rfc-0001
functor relation add rfc-0001 depends_on rfc-0000
functor query dependencies rfc-0001
functor validate
```

## Continuous Integration

Run:

- Unit tests
- Integration tests
- Formatting checks
- Linting
