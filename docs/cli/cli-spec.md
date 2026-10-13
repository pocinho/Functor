# CLI Specification

Status: Accepted
Related RFCs: RFC-0011 (projection), RFC-0002, RFC-0006, RFC-0007, RFC-0017
Owner crate: `functor-cli` (binary `functor`)
Supersedes: the earlier `command-design.md` draft (removed)
Related docs: [error-handling.md](../architecture/error-handling.md), [identity-and-addressing.md](../design/identity-and-addressing.md), [query-model-mvp.md](../design/query-model-mvp.md), [validation-rules.md](../design/validation-rules.md), [event-model-mvp.md](../design/event-model-mvp.md)

## Purpose

The exact command surface, arguments, output and exit codes of the MVP CLI. The CLI is a **projection**: it parses arguments, calls the core API and renders results. It contains no domain logic (RFC-0002) and is the composition root that wires concrete implementations ([ADR-0010](../adr/ADR-0010-async-trait-and-dyn.md)).

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
| `-v, --verbose` | | off | Show the cause chain of errors (`caused by:` lines, text mode) |
| `-q, --quiet` | | off | Suppress the confirmation of mutating commands in `text` mode; results (queries, `validate` reports, `watch` events) are always printed |
| `-h, --help`, `-V, --version` | | | Standard |

Precedence: command line over environment over default. Additional environment variable: `FUNCTOR_LOCK_TIMEOUT` (seconds to wait for the repository write lock, default 5; see [storage-design.md](../design/storage-design.md)). `init` uses `--repo` or its `[PATH]` argument as the **target directory**, not for discovery.

Commands that need a repository discover it by walking up from the current directory (or `--repo`). Failure: exit 5 `not a functor repository`.

Output conventions:

- **stdout** carries command results only. **stderr** carries diagnostics, warnings and errors. In `json` mode, stdout holds exactly one JSON document per command (JSON Lines for `watch`), and stderr holds `warning:` lines and error JSON (see [error-handling.md](../architecture/error-handling.md)).
- `text` output is for humans and may change between minor versions; **`json` output is the stable contract** and follows the schemas below. Golden tests cover both.
- JSON: UTF-8, keys in the order shown, arrays in the documented deterministic order, timestamps as `YYYY-MM-DDThh:mm:ssZ`, absent optional values as `null`. Every document has `"schema_version": 1` first.
- Output is plain text: the MVP uses no color and no ANSI sequences.

## Artifact Arguments

`<artifact>` accepts a canonical reference, an id or a slug (see [identity-and-addressing.md](../design/identity-and-addressing.md)). `<relation>` accepts a relationship id or canonical reference.

## Commands

### `functor init [PATH]`

Create a repository at `PATH` (default: the current directory; created if missing).

| Option | Description |
|--------|-------------|
| `--name <name>` | Repository name (default: directory name) |
| `--description <text>` | Optional description |
| `--nested` | Allow creating a repository inside another repository |

Creates what [repository-layout.md](../design/repository-layout.md) lists. Fails with exit 4 if `.functor/` already exists at `PATH` (init is not idempotent; no `--force` in the MVP). Fails with exit 4 if an ancestor is already a repository and `--nested` is not given. Text: `initialized repository "<name>" (<repo id>) at <path>`.

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

List artifacts ordered by slug. Options: `--type <type>`, `--tag <tag>`, `--long` (also show ids and update times). The same artifacts and output as `query artifacts` with the `--type` and `--tag` filters; `query` adds `--text` and `--meta`.

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

At least one change option is required (exit 2). Increments `revision`. The stored file is edited in place, so comments, formatting and unknown keys are preserved ([ADR-0011](../adr/ADR-0011-toml-edit-preserving-updates.md)). Emits `artifact.updated` listing the changed fields. If nothing actually changed, the command succeeds, writes nothing, emits nothing and says `no changes`.

### `functor relation add <SOURCE> <TYPE> <TARGET>`

Create `SOURCE --TYPE--> TARGET`. `TYPE` is `depends_on`, `references`, `derived_from` or `extends` (kebab-case aliases accepted). Other well-formed names require `--allow-custom-type`. Option `--meta <key=value>` (repeatable).

Errors: unknown artifact exit 3; self-reference of an acyclic type, bad type name or a type that is not built in without `--allow-custom-type`, bad `--meta` exit 2; duplicate `(source, type, target)` exit 4. The type and metadata are checked before the artifacts are looked up. Nothing is written on any error.
Emits `relationship.created`. Text: `added <rel id>: <source slug> <type> <target slug>`.

Note: argument order is `source type target`, so `functor relation add rfc-0001 depends_on rfc-0000` means rfc-0001 depends on rfc-0000.

### `functor relation list`

List relationships ordered by relationship id. Options: `--type <type>` (repeatable, kebab-case accepted, any well-formed name), `--source <artifact>`, `--target <artifact>`. Filters combine with AND; several `--type` values combine with OR. `--source` and `--target` take an artifact id, canonical reference or slug; an unknown artifact is exit 3, not an empty list. Text columns: `ID  SOURCE  TYPE  TARGET` (slugs); empty result: `no relationships found`.

### `functor relation show <RELATION>`

Show one relationship. `<RELATION>` is a `rel_...` id or a canonical reference; a slug or an artifact id is exit 2, an unknown relationship exit 3. Text: `id`, `type`, `source` and `target` (each `<slug> (<artifact id>)`), `created` and, when present, `metadata`.

### `functor query ...`

All `query` subcommands are read-only and emit no events. Semantics: [query-model-mvp.md](../design/query-model-mvp.md).

| Subcommand | Description |
|------------|-------------|
| `query artifacts` | Options `--type`, `--tag`, `--text <substr>`, `--meta <key=value>` (repeatable) |
| `query dependencies <ARTIFACT>` | What the artifact depends on. Options `--depth <n\|all>` (default `1`), `--type <t>` (repeatable, default `depends_on`) |
| `query dependents <ARTIFACT>` | What depends on the artifact. Same options |
| `query relationships <ARTIFACT>` | Every relationship touching the artifact. Options `--type` (repeatable), `--direction <in\|out\|both>` (default `both`) |

Output and errors:

- `query artifacts` prints the same table and JSON document as `artifact list` (`SLUG TYPE REV TITLE`); no match prints `no artifacts found`. Malformed criteria are exit 2.
- `query dependencies|dependents` print a heading `<dependencies|dependents> of <slug> (<types>, depth <n|all>)` and a table `DEPTH SLUG TYPE TITLE` ordered by depth, slug, id; no match prints `no dependencies found` or `no dependents found`. `--depth` is a whole number (0 allowed) or `all`; `--type` replaces the default `depends_on`, is repeatable and accepts the kebab-case alias. An unknown artifact is exit 3 with suggestions.
- `query relationships` prints the table of `relation list` (`ID SOURCE TYPE TARGET`) ordered by relationship id; none prints `no relationships found`.
- `-q` does not silence query results. A repository with a dangling relationship fails every query with exit 5 (`inconsistent`).

### `functor validate [ARTIFACT]`

Validate the repository ([validation-rules.md](../design/validation-rules.md)). Options: `--strict` (warnings also fail). Exit 0 for `passed` and `warning`; exit 6 for `failed`, `unknown` and (with `--strict`) `warning`. Emits `validation.completed`.

- **A report is a result, not an error.** It is printed to stdout (text or JSON) even when the exit code is 6; stderr stays empty. Errors (`NotARepository`, `NotFound` for the scoped artifact, `InvalidInput`, storage) are printed to stderr as for every command.
- **Damaged repositories are validated, not refused.** `validate` finds the repository root by `.functor/repository.toml` without parsing it, so an unsupported `format_version` (`V0013`) or a malformed `repository.toml` (`V0001`) is a finding, while every other command still fails with exit 5.
- **Text** has one line per finding, `<severity> <code> <subject>: <message>` (subject: `repository`, `artifact <id>`, `relationship <id>` or `file <path>`), an indented `related:` line when the finding has related subjects, then `validated <n> artifacts, <m> relationships` and `outcome: <outcome> (<e> errors, <w> warnings[, <i> infos])`; a clean run prints `outcome: passed` without the parenthesis. With `[ARTIFACT]` the second line ends with `(findings about <ARTIFACT>)`.
- **JSON** is the `validate` document below, without a timestamp, so one repository always produces the same bytes (also across clocks and id sources).
- **`[ARTIFACT]`** takes an id, canonical reference or slug and restricts the reported findings as described in [validation-rules.md](../design/validation-rules.md#scoped-validation). An artifact that does not exist, or did not load cleanly, is exit 3.
- `-q` does not silence the report.

### `functor watch`

Stream repository events ([event-model-mvp.md](../design/event-model-mvp.md)).

| Option | Description |
|--------|-------------|
| `--from-start` | Include all events currently in the log |
| `--after <seq>` | Events with `seq` greater than N |
| `--type <prefix>` (repeatable) | Filter by event type or prefix, e.g. `artifact.` |
| `--subject <artifact>` | Only events about this artifact |
| `--count <n>` | Exit 0 after N events |
| `--timeout <seconds>` | Exit 0 after this many seconds with no new events |

Text output, one line per event: `<seq>  <at>  <type>  <summary>`, where the summary is the slug for artifact events, `<source slug> <type> <target slug>` for `relationship.created` and the outcome for `validation.completed`. Slugs are resolved when the line is printed; an artifact that no longer resolves prints its id.

Without `--count` and `--timeout` it runs until interrupted (exit 0). `--from-start` and `--after` are mutually exclusive. `--count` must be at least 1 and counts only events that pass the filters. `--timeout` is an **idle** timeout: it restarts every time an event arrives, so `watch --timeout 3` keeps running while events keep coming at least every 3 seconds.

Behavior and errors:

- Results (event lines) go to stdout and are flushed after each line; warnings go to stderr as `warning: <text>` (events that were pruned before the start position, an unreadable line that was skipped, events of an unsupported envelope version). In `json` mode each line is the envelope exactly as stored in the log.
- An unknown `--subject` is exit 3. An unreadable log is exit 7. Outside a repository is exit 5. A closed output pipe ends the command quietly with exit 0, as does Ctrl-C.
- `watch` opens the repository (so a damaged `repository.toml` is exit 5, as for other commands) but never takes the write lock and never writes.
- `-q` does not silence `watch`.

## Exit Codes

Defined in [error-handling.md](../architecture/error-handling.md): 0 success, 1 internal, 2 usage or input, 3 not found, 4 conflict, 5 not a repository or unsupported format, 6 validation failed, 7 storage or lock.

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
validated 2 artifacts, 1 relationship
outcome: passed
```

With findings:

```text
$ functor validate
error V0007 relationship rel_01J9Z3KQ0A1B2C3D4E5F6G7H8J: target art_01J9Z3K0P4Q5R6S7T8V9W0X1Y2 does not exist
validated 2 artifacts, 1 relationship
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
| `query dependencies`, `query dependents` | `{"schema_version":1,"start":ArtifactRef,"direction":"outbound"\|"inbound","relationship_types":[...],"max_depth":n\|null,"nodes":[{"depth","artifact":Artifact,"via":Relationship}],"edges":[Relationship]}` |
| `query relationships` | `{"schema_version":1,"artifact":ArtifactRef,"relationships":[Relationship]}` |
| `validate` | `{"schema_version":1,"outcome":"passed"\|"warning"\|"failed"\|"unknown","strict":bool,"counts":{"artifacts","relationships","errors","warnings","infos"},"findings":[Finding]}`; a subject is `{"kind":"repository"}`, `{"kind":"artifact"\|"relationship","id"}` or `{"kind":"file","path"}` |
| `watch` | one event envelope per line, exactly as stored in the log |

Schema changes: adding fields is compatible; removing or renaming fields or changing meaning bumps `schema_version`. A JSON Schema file per document lives in `docs/cli/schemas/` and is used by the tests. Present so far: `init.json`, `error.json`, `artifact-document.json`, `artifact-list.json`, `relationship-document.json`, `relationship-list.json`, `query-traversal.json`, `query-relationships.json`, `validate.json` (shared definitions are kept identical by tests); the rest arrive with their commands.

## Implementation Notes (M4)

- **Verbosity and color.** `-v` prints the error cause chain (`caused by:` lines, text mode only) and `-q` silences confirmations in `text` mode only. Color, `--no-color`, `NO_COLOR`, `RUST_LOG` and `-vv` are not part of the MVP (decision recorded in [cli-mvp-roadmap.md](../planning/cli-mvp-roadmap.md#decisions-closed-before-m9)).
- **Events:** `artifact create`/`update` build their events and append them through the recorder (see Implementation Notes (M8)).
- **`artifact show`:** text mode prints labeled fields; with `--content` it strips one trailing newline and escapes control characters except newline and tab. JSON `content.text` is present for inline content and for `show --content` (referenced file text), otherwise absent.
- **`artifact list`:** sorted by slug, then id; `--long` adds id and updated time.
- **`--content-path`:** resolved against the current directory (not the repository root), canonicalized, and must be an existing file inside the repository. `--content-format` without content is exit 2.
- **Test-only variables** `FUNCTOR_TEST_FIXED_TIME` and `FUNCTOR_TEST_ID_SEED` exist only with the `test-support` feature ([testing-strategy.md](../quality/testing-strategy.md)).

## Implementation Notes (M5)

- **Endpoint slugs are always present.** The store refuses a repository whose relationship points at a missing artifact (exit 5, `inconsistent`, with a hint to run `functor validate`), so `relation add|list|show` never see a dangling endpoint and the `slug` of a JSON endpoint is always a string. Reading and reporting such repositories is the job of `validate` (M7).
- **Duplicate message.** The store reports a duplicate with artifact ids rather than slugs (`depends_on already relates art_… to art_…`), because it knows no slugs. Kept as is by decision: ids are unambiguous and scriptable.
- **Events:** `relation add` builds `relationship.created` and appends it like the artifact commands.
- **Not yet checked at add time:** cycles across several relationships. They are reported by `validate` (V0011, M7).

## Implementation Notes (M6)

- **Engine.** The `query` commands call `functor-query` through `SnapshotQueryEngine`; see [query-model-mvp.md](../design/query-model-mvp.md#implementation-notes-m6) for the semantics decisions (edge listing, text and metadata matching, one snapshot per command through the cache).
- **`--meta` in queries is a string match.** It finds string metadata and members of string lists; integer and boolean metadata written by hand do not match `--meta key=3`.
- **Schemas.** `query-traversal.json` serves `dependencies` and `dependents`; `query-relationships.json` serves `relationships`; `query artifacts` uses `artifact-list.json`.

## Implementation Notes (M7)

- **`validate` opens the repository differently.** `context::open_unchecked` walks up to the nearest `.functor/repository.toml` (`functor_repository::discover_root`) and builds the store with `FileStore::at_root`, without parsing `repository.toml`. Other commands keep the checking `discover`.
- **`validation.completed`.** `validate` appends the event (outcome, error, warning and info counts, `strict`) like the other commands do, unless the repository identity is unreadable.
- **Not found with suggestions.** The engine reports an unknown scoped artifact as a plain `NotFound`; the command adds slug suggestions from a lenient load.
- **Plurals.** Counts are pluralized (`1 relationship`, `2 relationships`); the earlier draft showed `1 relationships`.

## Implementation Notes (M8)

- **Recorder.** Commands hand the event they built to `events::Recorder`, which appends it through `functor_events::FileEventLog` after the state change committed. A failed append prints `warning: event not recorded: <reason>` to stderr (also in `json` mode, where stdout still holds exactly one document) and the command keeps its exit code; a rotation or pruning problem prints its own `warning:` line. `validate` still exits 6 for a failed outcome even when its event cannot be recorded.
- **Event ids and the id source.** An event takes its id from the same injected generator as entities, after the entity id, so the deterministic test ids stay stable.
- **Runtime.** The command line builds its single-threaded runtime with timers and signal handling enabled, which `watch` needs; `tokio::select!` is not used (a small `race` helper in `commands/watch.rs` ends the command on Ctrl-C), so `tokio` needs no `macros` feature outside tests.
- **Slug cache.** `watch` resolves slugs through one cached listing, refreshed when an id is missing and cleared by `artifact.updated`, so renames appear in later lines without a listing per event.

## Implementation Notes (M9)

- **One read per command.** The command line wraps its `FileStore` in `CachingStore`, so every command that reads the repository more than once (resolving two endpoints, resolving a start artifact and then traversing, suggestions after a failed lookup) loads it once. `validate` and `watch` bypass the cache on purpose: `validate` uses the lenient `load()` of an uncached store, and `watch` outlives any snapshot. See [storage-design.md](../design/storage-design.md#implementation-notes-m9).
- **Acceptance.** `tests/acceptance.rs` runs the walkthrough of [getting-started.md](getting-started.md) with golden transcripts, and `tests/library.rs` runs it through the libraries ([testing-strategy.md](../quality/testing-strategy.md#acceptance-test)).

## Logging

The MVP has no logging framework: diagnostics are the `error:` and `warning:` lines on stderr and, with `-v`, the cause chain of an error. Structured logging (`tracing`, `RUST_LOG`, `-v`/`-vv` levels) and color are post-MVP.

## Compatibility With Earlier Drafts

The earlier `command-design.md` listed `graph`, `runtime`, `protocol`, `agent`, `doctor`, `status` and `config` commands, and `artifact delete`. None are in the MVP. They return when the relevant RFCs are implemented (graph visualization and `doctor` are the next candidates).

## Test Obligations

- `assert_cmd` end-to-end tests for every command, covering text and json output, every exit code and every error variant.
- Golden files (`insta` snapshots) for the examples above, using a fixed clock and deterministic ids through the test-only environment variable `FUNCTOR_TEST_FIXED_TIME` and `FUNCTOR_TEST_ID_SEED`. These variables are compiled only under the `test-support` cargo feature and **must not exist in release builds**.
- JSON outputs validated against `docs/cli/schemas/*.json`.
- Argument-grammar tests (clap `debug_assert`).
