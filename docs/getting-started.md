# Getting Started (MVP Walkthrough)

Status: Accepted. This walkthrough is the acceptance script described in [testing-strategy.md](testing-strategy.md): the CLI acceptance test (`src/app/cli/tests/acceptance.rs`) runs these commands and compares the output with golden transcripts. The ids and times below are the deterministic ones of the test and of the reference repository in [`examples/getting-started/`](../examples/getting-started/); on your machine the ids are different ULIDs and the times are the current time.

## 1. Create a repository

```bash
mkdir functor-demo && cd functor-demo
functor init --name "Functor Demo"
```

```text
initialized repository "Functor Demo" (repo_00000000000000000000000001) at /home/user/functor-demo
```

You now have `.functor/repository.toml`, `artifacts/`, `relationships/` and `events/` (see [repository-layout.md](repository-layout.md)).

## 2. Create artifacts

```bash
functor artifact create rfc-0000 --type rfc --title "Functor Vision"
functor artifact create rfc-0001 --type rfc --title "Core Semantic Model"
```

```text
created artifact rfc-0000 (art_00000000000000000000000034)
created artifact rfc-0001 (art_00000000000000000000000068)
```

`rfc-0000` is a **slug**: a human-friendly alias. The opaque id is the identity ([identity-and-addressing.md](identity-and-addressing.md)).

## 3. Relate them

```bash
functor relation add rfc-0001 depends_on rfc-0000
```

```text
added rel_0000000000000000000000009C: rfc-0001 depends_on rfc-0000
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

```text
dependents of rfc-0000 (depends_on, depth 1)
DEPTH  SLUG      TYPE  TITLE
1      rfc-0001  rfc   Core Semantic Model
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
functor watch --from-start --count 4
```

```text
1  2026-10-12T18:00:00Z  artifact.created  rfc-0000
2  2026-10-12T18:00:05Z  artifact.created  rfc-0001
3  2026-10-12T18:05:00Z  relationship.created  rfc-0001 depends_on rfc-0000
4  2026-10-12T18:06:00Z  validation.completed  passed
```

## 7. Use JSON from scripts

```bash
functor query dependencies rfc-0001 --output json | jq '.nodes[].artifact.slug'
```

## Reference repository

[`examples/getting-started/`](../examples/getting-started/) is the final state of this walkthrough. A test compares it with what the walkthrough produces, so this document and the example cannot drift from the product. After a deliberate change, regenerate it with `FUNCTOR_UPDATE_EXAMPLE=1 cargo test -p functor-cli --all-features --test acceptance` and review the diff; the golden transcripts are updated as described in [testing-strategy.md](testing-strategy.md).
