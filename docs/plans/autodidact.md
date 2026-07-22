# Autodidact — Archetect teaches its own drivers

## Status snapshot (2026-07-21)

| Phase | Scope | Status |
|---|---|---|
| 0 | Feature audit (this document, §1) | done |
| 1 | MCP identity: `get_info` instructions + embedded skill | planned |
| 2 | `archetect learn [topic]` + MCP `learn {topic?}` + `archetect://learn/<topic>` resources | planned |
| 3 | `archetect introspect [filter]` / MCP `introspect` computed from the embedded LuaLS stubs | planned |
| 4 | `archetect eval '<lua>'` — one-shot probe of the scripting environment | planned |
| 5 | Slot-computed "this environment" sections (catalog, locals, cache, config) | planned |
| 6 | MCP parity gaps (cache/check/config tools, entry-level flags in `catalog_render`) | planned |
| 7 | Outward flow: docs-site reference pages regenerated from the same sources | planned |

This is the Archetect port of prova's autodidact system
(`prova-rs/prova docs/plans/autodidact.md`, shipped in prova 2026-07-21). The two systems share
a root — Rust + mlua, LuaCATS annotation stubs + `ide setup` + `.luarc.json`, manifest-driven
packages, git-source caching (literally a shared content-addressed cache via
`archetect-git-cache`'s normalized repo hash), headless-first automation flags, MCP crates, jj,
xtask — so the port is mostly a mapping, not a redesign. Governing principle, unchanged:
**computed > generated > hand-written**. The binary is the source of truth; prose that restates
what the binary knows is a drift liability (see
`docs/plans/documentation-audit-and-rewrite.md` for what hand-written docs did without this
rail: hallucinated enum variants, fictional Rhai support, invalid examples).

## 1. What exists today (audit, 2026-07-21)

### The feature surface an agent must learn

**CLI** (18 verbs, `archetect-bin/src/cli.rs:13`, dispatch `main.rs:147`): `render`, bare
catalog dispatch (`archetect [action]`), `global`, `ls`/`list`, `search`/`find`,
`config merged|defaults|edit`, `cache clear|pull|invalidate|prune`, `check`, `ide setup`,
`system layout`, `mcp`, `server`, `connect`, `completions`. The automation trio:
`--headless` + `-a/--answer`/`-A` + `-D/--use-defaults-all` (with `-s/--switch`,
`-n/--dry-run`, `-o/--offline`, `-U/--force-update`, `--allow-exec`, `-l/--local`).

**Scripting** (Lua via mlua; `script/lua/modules.rs`, `require_modules.rs`): `Context`
(prompt_text/int/confirm/select/multiselect/list/editor + get/set/merge, case expansion),
`Case`/`Cases`, `archetect`/`archetype` self-inspection, `catalog.render`,
`directory.render`, `file`, `template` (+ `register_filters`), `format`, `log`, `output`,
`exit`, `Location`/`Existing` enums; `require("archetect.shell|git|github|archive|model|
model.interactive")`. Sandboxed VM, `ShellExecPolicy` gates exec.

**Templates**: ATL — a custom compile-to-Lua template language (`templating/atl/`), `{{ expr
| filter }}` + `{% lua %}`, compile-time includes, filter/function symmetry, ~20 case and
inflection filters + string/collection/datetime/path/uuid builtins, strict/lenient undefined
modes.

**Authoring model**: `archetype.yaml` manifest (description/summary/authors/languages/
frameworks/tags, `requires.archetect` major-gated version floor, `catalog:` map,
`templating:`, `interface:` declarative prompt contract, optionally as sibling
`interface.yaml`); fixed `archetype.lua` entry point + `lib/`; catalogs are manifests all the
way down (leaf `source:` / group `catalog:` / federated `server:` entries, per-entry
`answers`/`switches`/`use_defaults(_all)`/`library`/`show`); composition via
`catalog.render` with three-layer flag overlay (inherited → opts → entry); `library: true`
staging into `package.path`; AML model-driven generation (`archetect-aml`).

**Sources & cache**: git URLs/SSH shorthand with `#ref`, locals override, content-addressed
per-commit trees with leases and TTL/hash freshness gates (`archetect-git-cache`).

**MCP** (`archetect-mcp`, 6 tools): `render`, `respond`, `cancel` (turn-based prompt session
via `PromptEnvelope`), `catalog_browse`, `catalog_search`, `catalog_render`. Shell exec
forced Forbidden. One session at a time.

### What is missing (the autodidact gaps)

1. **The MCP server has no identity.** `impl ServerHandler for ArchetectMcpServer {}` is
   empty (`server.rs:56`) — no instructions, no resources, no prompts. A connected agent
   gets six bare tool names and must guess the practice.
2. **Zero runtime introspection.** No `help()` in Lua, no `learn`/`docs`/`introspect` verb.
   Discoverability is editor-only (LuaLS stubs via `ide setup`) — invisible to an agent
   driving the CLI or MCP. The stubs ARE embedded in the binary
   (`ide_subcommand.rs:9-10`) — the raw material for computed introspection already ships.
3. **No probe loop.** Prova's crash course leans on `prova eval` ("never guess, ask the
   binary"). Archetect has no way to run one Lua snippet against the scripting environment
   without authoring a whole archetype.
4. **MCP surface is narrower than the CLI** — no cache/check/config/system access, no
   dry-run/offline/answer-file, and `catalog_render` skips entry-level answers/switches
   (`server.rs:486` TODO). An MCP agent must shell out and cannot discover that rule.
5. **The docs site drifted into fiction** because nothing computed pushed back
   (`documentation-audit-and-rewrite.md`). Same disease prova's §2.9 outward flow cures.

## 2. Design

Identical three-surface shape to prova: one renderer, three access rails —
CLI (`archetect learn`), MCP tool (`learn {topic?}`), MCP resources
(`archetect://learn/<topic>` + `archetect://skill`). Embedded markdown topics with
closed-enum `{{slot}}` dynamic sections computed from the resolved configuration.
Transport-conditioned rendering: MCP output spells tools, CLI output spells commands.

### 2.1 Topic taxonomy (13 topics, one screen each)

| Topic | Teaches | Prova analog |
|---|---|---|
| `generation` | the practice: render, don't hand-write; the agent loop (search → dry-run → render headless → verify, ideally with prova); never guess — introspect/eval | `pdd` |
| `environment` | THIS machine: configured catalog root + first entries, locals, cache state, annotations installed, project `.archetect.yaml` | `project` |
| `rendering` | render/global/bare-dispatch verbs; the automation trio; flag bags & `name=false` overlay semantics; dry-run | `running` |
| `authoring` | `archetype.lua` + `Context` in one screen; prompt → set → render flow; `lib/` helpers | `authoring` |
| `manifest` | every `archetype.yaml` key, one line each; `requires` major-gate; `interface:` contract | `project`(manifest half) |
| `prompts` | 7 prompt types + options; the headless resolution order (answer → default → optional → error); `interface.yaml` as the declarative mirror | — |
| `cases` | Case/Cases, case expansion on set/prompt, the inflection filter family | — |
| `templates` | ATL syntax, filters/functions, strict mode, includes, `register_filters` | — |
| `catalogs` | entry kinds (leaf/group/server), per-entry flags, source addressing, `show`/`library` | `init` |
| `composition` | `catalog.render`, three-layer flag propagation, library staging, `require` paths | `plugins` |
| `model` | AML: schema, `archetect.model` query API, when to reach for it (marked partly unshipped where true) | `proxies`(honesty pattern) |
| `sources` | git/ssh/`#ref`/locals/offline; the content-addressed cache and its verbs | — |
| `mcp` | driving archetect over MCP: session model, PromptEnvelope/respond loop, what to shell out for | `mcp` |

Aliases (`case`→`cases`, `template`→`templates`, `catalog`→`catalogs`, `aml`→`model`, …).
Project-provided docs join the rail exactly like prova's `context = [...]` key → `ctx:*`
topics (candidate key on `.archetect.yaml` or `archetype.yaml`).

### 2.2 Introspection: computed from the stubs that already ship

Port prova's `help.rs` LuaCATS parser (it already parses `---@meta` stubs into
name/signature/summary entries — the two stub files here are the same dialect). Surfaces:
`archetect introspect [filter]` (CLI), `introspect {filter}` (MCP), and — inside scripts —
a `help("<filter>")` global, so an archetype author mid-`eval` can ask for shapes. Because
both projects parse the same stub dialect, extracting the parser into a small shared crate
(`archetect-common` org or a `lua-help` crate) is the natural end-state; start by porting.

### 2.3 `eval` — the probe verb

`archetect eval '<lua>'` (and MCP `eval {code}`): one-shot script in a synthetic archetype
root (temp dir, no manifest required), full module surface, `--headless` semantics, values
returned as YAML/JSON. This is what turns "read the docs" into "ask the binary" for
template filters (`eval 'return template.render("{{ x | train_case }}", {x="foo bar"})'`),
case specs, model queries, and manifest questions. Dry-run by default; `--allow-exec` opts
into shell/git effects exactly as render does.

### 2.4 MCP identity + parity

- `get_info` → instructions = the embedded skill (the same file `archetect skill` prints and
  `skill --install` writes for Claude/agents; mirror prova's transport table: iterate over
  MCP, shell out for `ide setup`/`cache`/CI).
- Resources: `archetect://learn/<topic>`, `archetect://skill`.
- Close the parity gaps the audit found: `catalog_render` applies entry-level
  answers/switches (route through `catalog::dispatch` instead of reimplementing);
  add `check {}`/`cache {pull|invalidate|prune}`/`config {merged}` tools or document the
  shell-out rule in the `mcp` topic — whichever, the topic states the truth.
- `render`/`catalog_render` gain `use_defaults` (per-key) and `answer_files`? — decide, then
  the topic documents what shipped, never what's intended.

### 2.5 Slots (closed enum, computed per invocation)

`CatalogTree` (root + first level of the resolved catalog, with the 🛰️ marker),
`Locals` (enabled + paths), `CacheState` (counts/size/retention), `ProjectConfig`
(`.archetect.yaml` found or not, switches), `Annotations` (installed or `ide setup` hint),
`InterfaceOf(<source>)`? (deferred — per-archetype, not per-environment). Degrade with a
one-line placeholder when no config is in reach (prova's lesson: never render a bare
heading — an empty section reads as a bug).

### 2.6 Enforcement ladder

- Closed `Topic`/`Slot` enums; a topic that names an unshipped feature marks it unshipped
  in the topic itself (the `model` topic inherits prova's `proxies` honesty pattern).
- clap already IS the verb table (unlike prova's hand-written HELP) — add a selftest that
  every CLI verb appears in exactly one topic, so a new verb cannot ship untaught.
- The `interface:` contract is the archetype-side analog: a prompt not declared there is
  invisible to tooling. Lint (in `check` or a future `archetype lint`) warns when an
  `archetype.lua` prompts for keys the interface omits.
- Selftest suite mirrors prova's `selftest/learn_test.lua` — and since prova is the
  ecosystem's black-box tester, these ship as **prova proofs** driving the archetect binary
  (the `archetect` namespace prova already exposes makes this near-free).

### 2.7 Outward flow

The docs-audit plan (`documentation-audit-and-rewrite.md`) fixes the site by hand once;
this plan keeps it fixed: an `xtask docs-export` renders the topic sources + introspect
entries into the site's reference pages, so the site can only drift in prose, never in API
fact. Sequence AFTER the audit's systemic fixes land, so we export truth, not fiction.

## 3. Milestones (proofs-first, prova as the harness)

- **M0 — skill + MCP identity.** Embed the skill (write it against §1's audited surface),
  serve it via `get_info` instructions + `archetect skill [--install]`. Proof: MCP
  initialize returns instructions containing the loop.
- **M1 — learn engine.** Port prova's `learn.rs` (Topic/Slot enums, alias resolution,
  transport-conditioned rendering), author the 13 topics. Proof: every topic renders under
  both transports; slot placeholders outside any config.
- **M2 — introspect.** Port `help.rs`; CLI + MCP + in-script `help()`. Proof: parity test —
  every function the stubs declare is answerable; every registered module appears in stubs
  (the drift-killer both directions).
- **M3 — eval.** Proof: probe returns filter output headlessly; exec-gated ops refused
  without `--allow-exec`.
- **M4 — MCP parity + slots.** Entry-level flags in `catalog_render`; slots live. Proof:
  prova proofs driving `archetect mcp` over stdio.
- **M5 — docs-export.** After the doc audit's systemic pass.

## 4. Open questions

1. ~~Where does the project-context key live?~~ **Decided 2026-07-21: `.archetect.yaml`**
   (the consumer's config, matching prova's precedent — the environment's owner curates
   what the agent learns). Archetype-shipped context (`archetype.yaml` docs entering the
   learn rail from a fetched source) is deferred as its own feature: text from a remote
   source landing in an agent's instruction channel needs a trust design first.
2. ~~Shared `lua-help`/`learn` crate now or later?~~ **Decided 2026-07-21: after both
   ports stabilize** — port first, extract when the second consumer proves the seam.
3. Does `eval` belong on the server/`connect` surface too (remote probe)? Defer.
4. `archetype lint` as its own verb vs folding interface-drift checks into `check`?
