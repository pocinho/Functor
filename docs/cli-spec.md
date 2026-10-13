# CLI Specification

Status: Draft
Related RFCs: RFC-0011 (projection), RFC-0002, RFC-0006, RFC-0007, RFC-0017
Owner crate: `functor-cli` (binary `functor`)
Supersedes: `command-design.md` (now a pointer to this document)
Related docs: [error-handling.md](error-handling.md), [identity-and-addressing.md](identity-and-addressing.md), [query-model-mvp.md](query-model-mvp.md), [validation-rules.md](validation-rules.md), [event-model-mvp.md](event-model-mvp.md)

## Purpose

The exact command surface, arguments, output and exit codes of the MVP CLI. The CLI is a **projection**: it parses arguments, calls the core API and renders results. It contains no domain logic (RFC-0002) and is the composition root that wires concrete implementations ([ADR-0010](adr/ADR-0010-async-trait-and-dyn.md)).

## Principles

- Scriptable by default: deterministic output, meaningful exit codes, machine-readable mode.
- Human friendly: concise text output, actionable error messages.
- Agent friendly: every command has `--output json` with a stable schema.
- Noun-verb grammar: `functor <resource> <action> [arguments] [options]`, with the verb-only exceptions `init`, `query`, `validate` and `watch`.
- No interactive prompts in the MVP. Nothing blocks waiting for input.

## Global Options

| Option | Env | Default | Description |
|--------|-----|---------|-------------|
| `--repo <path>` | `FUNCTOR_REPO` | discover upwards from the current directory | Repository root (or any directory inside it) |
| `-o, --output <text\|json>` | | `text` | Output mode |
| `--no-color` | `NO_COLOR` | color only on a TTY | Disable ANSI colors |
| `-v, --verbose` | `RUST_LOG` | off | `-v` info, `-vv` debug logs on stderr; shows error source chains |
| `-q, --quiet` | | off | Suppress non-essential stdout in `text` mode (results of queries are still printed) |
| `-h, --help`, `-V, --version` | | | Standard |

Precedence: command line over environment over default. Additional environment variable: `FUNCTOR_LOCK_TIMEOUT` (seconds to wait for the repository write lock, default 5; see [storage-design.md](storage-design.md)). `init` uses `--repo` or its `[PATH]` argument as the **target directory**, not for discovery.

Commands that need a repository discover it by walking up from the current directory (or `--repo`). Failure: exit 5 `not a functor repository`.

Output conventions:

- **stdout** carries command results only. **stderr** carries diagnostics, warnings and errors. In `json` mode, stdout holds exactly one JSON document per command (JSON Lines for `watch`), and stderr holds `warning:` lines and error JSON (see [error-handling.md](error-handling.md)).
- `text` output is for humans and may change between minor versions; **`json` output is the stable contract** and follows the schemas below. Golden tests cover both.
- JSON: UTF-8, keys in the order shown, arrays in the documented deterministic order, timestamps as `YYYY-MM-DDThh:mm:ssZ`, absent optional values as `null`. Every document has `"schema_version": 1` first.
- Color is used only for `error:`/`warning:` prefixes and outcome words, never for information that is not also in the text.

## Artifact Arguments

`<artifact>` accepts a canonical reference, an id or a slug (see [identity-and-addressing.md](identity-and-addressing.md)). `<relation>` accepts a relationship id or canonical reference.

## Commands

### `functor init [PATH]`

Create a repository at `PATH` (default: the current directory; created if missing).

| Option | Description |
|--------|-------------|
| `--name <name>` | Repository name (default: directory name) |
| `--description <text>` | Optional description |
| `--nested` | Allow creating a repository inside another repository |

Creates what [repository-layout.md](repository-layout.md) lists. Fails with exit 4 if `.functor/` already exists at `PATH` (init is not idempotent; no `--force` in the MVP). Fails with exit 4 if an ancestor is already a repository and `--nested` is not given. Text: `initialized repository "<name>" (<repo id>) at <path>`.

### `functor artifact create <SLUG>`

Create an artifact with the given slug.

| Option | Default | Description |
|--------|---------|-------------|
| `--type <type>` | `document` | Artifact type |
| `--title <text>` | the slug | Title |
| `--description <text>` | none | |
| `--tag <tag>` (repeatable) | none | |
| `--meta <key=value>` (repeatable) | none | String metadata |
| `--content-text <text>` | none | Inline content |
| `--content-path <path>` | none | Reference an existing file inside the repository (not copied) |
| `--content-format <fmt>` | guessed from extension, else `text` | |

`--content-text` and `--content-path` are mutually exclusive. Errors: invalid slug or option value exit 2; duplicate slug exit 4; content path outside the repository or missing exit 2.
Emits `artifact.created`. Text: `created artifact <slug> (<id>)`.

### `functor artifact show <ARTIFACT>`

Show one artifact. `--content` includes inline content or the referenced file text (limited to 1 MiB). Exit 3 if not found.

### `functor artifact list`

List artifacts ordered by slug. Options: `--type <type>`, `--tag <tag>`, `--long` (also show ids and timestamps). Equivalent to `query artifacts` with the same filters but with a simpler column set.

### `functor artifact update <ARTIFACT>`

| Option | Description |
|--------|-------------|
| `--slug <slug>` | Rename the alias |
| `--title <text>` | |
| `--description <text>` / `--clear-description` | |
| `--tags <a,b,c>` | Replace the whole tag set (`--tags ""` clears) |
| `--meta <key=value>` (repeatable), `--unset-meta <key>` (repeatable) | Set or remove metadata keys |
| `--content-text`, `--content-path`, `--content-format`, `--clear-content` | Same as `create`, or remove |
| `--expect-revision <n>` | Fail with exit 4 if the stored revision differs |

At least one change option is required (exit 2). Increments `revision`. The stored file is edited in place, so comments, formatting and unknown keys are preserved ([ADR-0011](adr/ADR-0011-toml-edit-preserving-updates.md)). Emits `artifact.updated` listing the changed fields. If nothing actually changed, the command succeeds, writes nothing, emits nothing and says `no changes`.

### `functor relation add <SOURCE> <TYPE> <TARGET>`

Create `SOURCE --TYPE--> TARGET`. `TYPE` is `depends_on`, `references`, `derived_from` or `extends` (kebab-case aliases accepted). Other well-formed names require `--allow-custom-type`. Option `--meta <key=value>` (repeatable).

Errors: unknown artifact exit 3; self-reference of an acyclic type, bad type name exit 2; duplicate `(source, type, target)` exit 4.
Emits `relationship.created`. Text: `added <rel id>: <source slug> <type> <target slug>`.

Note: argument order is `source type target`, so `functor relation add rfc-0001 depends_on rfc-0000` means rfc-0001 depends on rfc-0000.

### `functor relation list`

List relationships ordered by relationship id. Options: `--type <type>` (repeatable), `--source <artifact>`, `--target <artifact>`.

### `functor relation show <RELATION>`

Show one relationship. Exit 3 if not found.

### `functor query ...`

All `query` subcommands are read-only and emit no events. Semantics: [query-model-mvp.md](query-model-mvp.md).

| Subcommand | Description |
|------------|-------------|
| `query artifacts` | Options `--type`, `--tag`, `--text <substr>`, `--meta <key=value>` (repeatable) |
| `query dependencies <ARTIFACT>` | What the artifact depends on. Options `--depth <n\|all>` (default `1`), `--type <t>` (repeatable, default `depends_on`) |
| `query dependents <ARTIFACT>` | What depends on the artifact. Same options |
| `query relationships <ARTIFACT>` | Every relationship touching the artifact. Options `--type`, `--direction <in\|out\|both>` (default `both`) |

### `functor validate [ARTIFACT]`

Validate the repository ([validation-rules.md](validation-rules.md)). Options: `--strict` (warnings also fail). Exit 0 for `passed` and `warning`; exit 6 for `failed`, `unknown` and (with `--strict`) `warning`. Emits `validation.completed`.

### `functor watch`

Stream repository events ([event-model-mvp.md](event-model-mvp.md)).

| Option | Description |
|--------|-------------|
| `--from-start` | Include all events currently in the log |
| `--after <seq>` | Events with `seq` greater than N |
| `--type <prefix>` (repeatable) | Filter by event type or prefix, e.g. `artifact.` |
| `--subject <artifact>` | Only events about this artifact |
| `--count <n>` | Exit 0 after N events |
| `--timeout <seconds>` | Exit 0 after this many seconds with no new events |

Text output, one line per event: `<seq>  <at>  <type>  <summary>`, where the summary is the slug for artifact events, `<source slug> <type> <target slug>` for `relationship.created` and the outcome for `validation.completed`. Slugs are resolved when the line is printed; an artifact that no longer resolves prints its id.

Without `--count` and `--timeout` it runs until interrupted (exit 0). `--from-start` and `--after` are mutually exclusive.

## Exit Codes

Defined in [error-handling.md](error-handling.md): 0 success, 1 internal, 2 usage or input, 3 not found, 4 conflict, 5 not a repository or unsupported format, 6 validation failed, 7 storage or lock.

## Text Output Examples (golden)

```text
$ functor artifact create rfc-0000
created artifact rfc-0000 (art_01J9Z3K8M2Q7X4V6B5N1R0T8WD)

$ functor relation add rfc-0001 depends_on rfc-0000
added rel_01J9Z3KQ0A1B2C3D4E5F6G7H8J: rfc-0001 depends_on rfc-0000

$ functor query dependencies rfc-0001
dependencies of rfc-0001 (depends_on, depth 1)
DEPTH  SLUG      TYPE      TITLE
1      rfc-0000  document  rfc-0000

$ functor validate
validated 2 artifacts, 1 relationships
outcome: passed
```

With findings:

```text
$ functor validate
error V0007 relationship rel_01J9Z3KQ0A1B2C3D4E5F6G7H8J: target art_01J9Z3K0P4Q5R6S7T8V9W0X1Y2 does not exist
validated 2 artifacts, 1 relationships
outcome: failed (1 error, 0 warnings)
```

Tables have a header row, columns separated by two spaces and are padded to the widest cell; no borders.

Empty results: `text` mode prints a single line `no <things> found` (for example `no artifacts found`); `json` mode prints the document with an empty array. Both exit 0.

## JSON Schemas (stable)

Shared objects:

```text
Artifact      {"id","ref","slug","type","title","revision","description","tags":[],"metadata":{},"content":null|{"format","text"|"path"},"created","updated"}
Relationship  {"id","ref","type","source":{"id","slug"},"target":{"id","slug"},"created","metadata":{}}
ArtifactRef   {"id","slug"}
Finding       {"code","severity","subject":{"kind","id"|"path"},"message","related":[Subject]}
```

`ref` is the canonical reference (`functor:artifact:art_...`).

| Command | Document |
|---------|----------|
| `init` | `{"schema_version":1,"repository":{"id","name","root","format_version"}}` |
| `artifact create`, `show`, `update` | `{"schema_version":1,"artifact":Artifact,"changed":[...]}` (`changed` only for `update`) |
| `artifact list`, `query artifacts` | `{"schema_version":1,"artifacts":[Artifact]}` |
| `relation add`, `show` | `{"schema_version":1,"relationship":Relationship}` |
| `relation list` | `{"schema_version":1,"relationships":[Relationship]}` |
| `query dependencies`, `query dependents` | `{"schema_version":1,"start":ArtifactRef,"direction":"outbound"\|"inbound","relationship_types":[...],"max_depth":1\|null,"nodes":[{"depth","artifact":Artifact,"via":Relationship}],"edges":[Relationship]}` |
| `query relationships` | `{"schema_version":1,"artifact":ArtifactRef,"relationships":[Relationship]}` |
| `validate` | `{"schema_version":1,"outcome":"passed"\|"warning"\|"failed"\|"unknown","strict":bool,"counts":{"artifacts","relationships","errors","warnings","infos"},"findings":[Finding]}` |
| `watch` | one event envelope per line, exactly as stored in the log |

Schema changes: adding fields is compatible; removing or renaming fields or changing meaning bumps `schema_version`. A JSON Schema file per document lives in `docs/schemas/` and is used by the tests. Present so far: `init.json`, `error.json`, `artifact-document.json`, `artifact-list.json` (the `Artifact` definition in the last two is kept identical by a test); the rest arrive with their commands.

## Implementation Notes (M4)

- **Not yet implemented:** `--no-color` (nothing is colored yet) and `tracing` logging; `-v` currently prints the error cause chain (`caused by:` lines, text mode only), `-q` silences confirmations in `text` mode only.
- **Events:** `artifact create`/`update` already build their events and hand them to a no-op recorder; the real publisher arrives in M8.
- **`artifact show`:** text mode prints labeled fields; with `--content` it strips one trailing newline and escapes control characters except newline and tab. JSON `content.text` is present for inline content and for `show --content` (referenced file text), otherwise absent.
- **`artifact list`:** sorted by slug, then id; `--long` adds id and updated time.
- **`--content-path`:** resolved against the current directory (not the repository root), canonicalized, and must be an existing file inside the repository. `--content-format` without content is exit 2.
- **Test-only variables** `FUNCTOR_TEST_FIXED_TIME` and `FUNCTOR_TEST_ID_SEED` exist only with the `test-support` feature ([testing-strategy.md](testing-strategy.md)).

## Logging

`tracing` with `RUST_LOG` or `-v`/`-vv`, to stderr only. Default level `warn`.

## Compatibility With Earlier Drafts

The earlier `command-design.md` listed `graph`, `runtime`, `protocol`, `agent`, `doctor`, `status` and `config` commands, and `artifact delete`. None are in the MVP. They return when the relevant RFCs are implemented (graph visualization and `doctor` are the next candidates).

## Test Obligations

- `assert_cmd` end-to-end tests for every command, covering text and json output, every exit code and every error variant.
- Golden files (`insta` snapshots) for the examples above, using a fixed clock and deterministic ids through the test-only environment variable `FUNCTOR_TEST_FIXED_TIME` and `FUNCTOR_TEST_ID_SEED`. These variables are compiled only under the `test-support` cargo feature and **must not exist in release builds**.
- JSON outputs validated against `docs/schemas/*.json`.
- Argument-grammar tests (clap `debug_assert`).
