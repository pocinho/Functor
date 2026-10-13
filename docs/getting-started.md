# Getting Started (MVP Walkthrough)

Status: Draft. Output below is the expected behavior once the MVP is implemented; it doubles as the acceptance script described in [testing-strategy.md](testing-strategy.md). Ids shown are examples.

## 1. Create a repository

```bash
mkdir functor-demo && cd functor-demo
functor init --name "Functor Demo"
```

```text
initialized repository "Functor Demo" (repo_01J9Z3JX9T8S7R6Q5P4N3M2K1H) at /home/user/functor-demo
```

You now have `.functor/repository.toml`, `artifacts/`, `relationships/` and `events/` (see [repository-layout.md](repository-layout.md)).

## 2. Create artifacts

```bash
functor artifact create rfc-0000 --type rfc --title "Functor Vision"
functor artifact create rfc-0001 --type rfc --title "Core Semantic Model"
```

```text
created artifact rfc-0000 (art_01J9Z3K0P4Q5R6S7T8V9W0X1Y2)
created artifact rfc-0001 (art_01J9Z3K8M2Q7X4V6B5N1R0T8WD)
```

`rfc-0000` is a **slug**: a human-friendly alias. The opaque id is the identity ([identity-and-addressing.md](identity-and-addressing.md)).

## 3. Relate them

```bash
functor relation add rfc-0001 depends_on rfc-0000
```

```text
added rel_01J9Z3KQ0A1B2C3D4E5F6G7H8J: rfc-0001 depends_on rfc-0000
```

## 4. Query

```bash
functor query dependencies rfc-0001
functor query dependents rfc-0000
```

```text
dependencies of rfc-0001 (depends_on, depth 1)
DEPTH  SLUG      TYPE  TITLE
1      rfc-0000  rfc   Functor Vision
```

## 5. Validate

```bash
functor validate
```

```text
validated 2 artifacts, 1 relationship
outcome: passed
```

## 6. Watch events (second terminal)

```bash
functor watch --from-start
```

```text
1  2026-10-12T18:00:00Z  artifact.created       rfc-0000
2  2026-10-12T18:00:05Z  artifact.created       rfc-0001
3  2026-10-12T18:05:00Z  relationship.created   rfc-0001 depends_on rfc-0000
4  2026-10-12T18:06:00Z  validation.completed   passed
```

## 7. Use JSON from scripts

```bash
functor query dependencies rfc-0001 --output json | jq '.nodes[].artifact.slug'
```

## Reference repository

When implementation starts, add the final state of this walkthrough as `examples/getting-started/`. It becomes a test fixture so the document cannot drift from the product.
