# Dynamic Interface — the prompts ARE the interface

## Status snapshot (2026-07-22)

| Phase | Scope | Status |
|---|---|---|
| 0 | Design (this document) + inventory of `interface.yaml` consumers | done |
| 1 | Prompt-surface completion: `pattern`, rich options, `group`/`ui` — enforced, not descriptive | **shipped 2026-07-22** (proofs: `proofs/interface/prompt_surface_test.lua`; gRPC carries option values only until phase 6). `group` was **removed 2026-08-18** — `context:section` replaced it, see [Pages & Sections](interface-pages-and-sections.md) |
| 2 | Shared `PromptEnvelope` (moved mcp → api) + `ProbeDriver` (default-path recording, switch recording, budget guard) | **shipped 2026-07-22** |
| 3 | Consumers: `archetect interface <source\|path>` CLI (`--json`, `--answers-template`), MCP `describe` | **shipped 2026-07-22** (browse still serves the declared interface; probe-result caching by commit deferred) |
| 4 | Drift detection (`--check`) + deprecation warning on declared interfaces | **shipped 2026-07-22** (clap-cli migration pending — it is the one ecosystem user) |
| 5 | Branch exploration (`--explore` / `explore:true`): per-decision forking, `appears_when`, computed batch/interactive | **shipped 2026-07-22** (per-decision coverage, not full cartesian; nested decisions get their own runs) |
| 6 | `DescribeArchetype` gRPC (JSON-payload v1, served from the catalog path, explore supported) | **shipped 2026-07-22** (proofs drive a live server over reflection; fixing that surfaced and fixed a prova gRPC-client reflection bug) |
| 7 | REMOVAL of the declared interface: `interface:` / `interface.yaml` are a hard load error naming the migration; `--check` retired with them; docs-site + learn topics + spec swept; clap-cli migrated (validated with `--check --explore` first, then deleted) | **shipped 2026-07-22** (sign-off given) |
| 8 | Remaining polish: probe-result caching by commit, typed proto carrying rich options over gRPC | planned |
| 9 | Answer-aware derivation: `-a`/`-A`/`-s` on `interface`, `answers`/`switches` on MCP `describe`, `answers_yaml`/`switches` on gRPC `DescribeArchetype` — the interface describes what is STILL unknown, and progressive re-describe paginates a conditional archetype with no session | **shipped 2026-08-19** (proofs: `proofs/interface/answer_aware_test.lua`) |

**Removal ripple:** the rust-clap-cli-archetype fix is committed locally but the remote
`#v1` tag still ships interface.yaml — renders of the REMOTE clap-cli error under the new
binary until that commit is pushed and the tag recut. Everything else in the ecosystem was
already declaration-free.

Acceptance bar: `proofs/interface/` (prova, black-box on the shipped binary) — 21 proofs
covering probe transcript fidelity, switch recording, zero side effects, composition
descent, loop budget, exploration/`appears_when`/batch classification, answers-template
round-trip, drift both ways, MCP describe parity, and the deprecation warning.

Governing principle, inherited from autodidact: **computed > generated > hand-written**.
The `interface:` manifest block and sibling `interface.yaml` are hand-written restatements
of facts the script already declares — every `ctx:prompt_*` call carries key, type, label,
help, placeholder, default, options, and constraints. The 2026-07-22 fresh-context
assessment found the declared and actual surfaces drifted in practice (`learn prompts`
taught a `defaults` option that doesn't exist; the documented switch example didn't parse),
and the runtime admits it cannot enforce agreement. **Explicit goal: delete
`interface.yaml`.** The interface becomes something you *ask the archetype*, not something
its author promises.

## 0. Three ways to drive an archetype

Answer-aware derivation (phase 9) turned a binary choice into three, and the third is the one
a wizard wants:

| Mode | How | Handles dynamism | Can lay out a form |
|---|---|---|---|
| **Interactive** | prompt by prompt over `StreamingApi` | yes — the script decides as it goes | no: never sees more than one field ahead |
| **Big bang** | one `describe`, render the whole interface | no: a prompt that only exists once another is answered cannot be in it | yes |
| **Hybrid** | `describe` → render the first unfinished page → collect → `describe` again | yes | yes, one page at a time |

The hybrid is a loop of **idempotent queries** — `describe(archetype, answers, switches)` is a
pure function, no session, no server state — and it terminates because every round either
answers something or finds nothing left to ask:

```
answers = {}
loop:
  iface = describe(answers)
  page  = first page in iface.layout with non-empty children   # empty == finished
  if none: render(answers); done                               # asks nothing
  answers += collect(page.children)
```

It copes with both shapes of conditional, which is why it is worth stating as an algorithm
rather than a convention:

- **cross-page** — a page that only exists down one branch is absent from the layout until an
  answer selects it, then appears in its declared position.
- **intra-page** — a prompt revealed by an answer in its OWN page brings that page back
  non-empty, so the loop stays on the step. It degrades to more rounds, never to breakage.

Two things a client must get right, both proven in
[`proofs/interface/wizard_loop_test.lua`](../../proofs/interface/wizard_loop_test.lua):

- **A finished page comes back EMPTY, not answered.** Render the layout's pages without
  checking and every completed step is a blank screen.
- **Key on the segment `key`, never on index** — pages appear and disappear between rounds.

The loop alone cannot say "step 2 of 4": a page behind an unselected branch is not in the
layout yet, so the count grows as the user walks. One `--explore` call up front gives the
superset in declaration order for a progress indicator — exploration for the SHAPE,
re-derivation for the CONTENT, and the client still never evaluates an `appears_when`.

## 1. Why this works: the architecture is already right

Every UI is an IO driver consuming the same `ScriptMessage` stream
(`archetect-api/src/io_driver.rs` — `ScriptIoHandle`): the terminal
(`archetect-terminal-io`), the MCP session (`archetect-mcp/src/io_handle.rs`), and the gRPC
remote-render path all render identical prompt messages and reply with `ClientMessage`s.
A web form is a fourth driver, not a new protocol.

Two consequences:

- **Interactive mode already exists everywhere.** Choose-your-own-adventure prompting is
  the substrate; any client that speaks the session protocol gets it, including branching
  the script only decides at runtime.
- **Interface mode is a recording.** Run the script against a driver that answers instead
  of asking, and the transcript of prompt envelopes IS the interface — same fidelity the
  terminal gets, delivered all at once. Side-effect suppression is free at this layer:
  the driver receives `WriteFile`/`WriteDirectory` and simply Acks without writing
  (the MCP driver already proves writes are the driver's decision, `session.rs`).

Batch degrades safely: a form generated from a probe that misses a prompt (stale cache,
divergent branch) doesn't fail the render — the session loop catches the unexpected prompt.
Interface mode is an optimization over an always-correct interactive substrate, so the
probe must be *useful*, never *perfect*.

## 2. Phase 1 — finish the prompt surface (prereq for deletion)

`interface.yaml` currently expresses three things prompts cannot. Move each into prompt
opts, where it stops being a claim and becomes a behavior:

| Declarative-only today | Prompt opt | Gained behavior |
|---|---|---|
| `validation: "^[a-z]…"` | `pattern` on `prompt_text` | enforced on every path: interactive, `-a`, answer files, MCP |
| `options: [{value,label,help}]` | options arrays accept tables alongside strings | terminal + web render labels; value is what's stored/answered |
| `groups:` | `group = "Identity"` shared opt | envelope carries it; ordering stays script order — *superseded by `context:section`, 2026-08-18* |

Plus a free-form `ui = { … }` shared opt: an opaque table recorded into the envelope and
passed through untouched (widget hints, `advanced = true`, icons). Intrinsic hints belong
in the script; org/theme presentation does NOT — it belongs in the consuming UI, not in a
resurrected overlay file.

Deliverables: opts on all seven prompt types where they make sense, runtime enforcement of
`pattern` (reject with the message naming key + pattern, same voice as the headless error),
LuaCATS stubs updated (introspect/IDE follow automatically), `learn prompts` updated.

## 3. Phase 2 — the InterfaceProbe driver

New `ScriptIoHandle` impl (archetect-core, alongside a `PromptEnvelope` **moved from
archetect-mcp into archetect-api** so CLI, MCP, gRPC, and probe share one envelope type):

- `send(PromptFor*)` → record envelope; `receive()` → auto-answer:
  answer if the key is answered (probe accepts `-a`/`-A` — probing *with* partial answers
  narrows conditional branches deliberately); else default; else type-synthetic
  (`""`, `0`, `false`, first option, `[]`); `optional` → `None`.
- `send(WriteFile|WriteDirectory)` → Ack, write nothing. Exec is forced off
  (`ShellExecPolicy` deny): a script that *requires* exec fails the probe → classified
  interactive, which is honest.
- **Switch recording**: `archetype.switches.is_enabled(name)` is in-process, not an IO
  message — `RenderContext` gains an optional recorder; the Lua switches module logs every
  queried name. Switch checks are typically unconditional, so even default-path probing
  closes the discoverability gap that motivated this plan (switches are never prompted, so
  the session loop can never reveal them).
- **Composition descends**: `catalog.render` children run under the same driver; their
  prompts join the transcript exactly as they'd join a real session (already-answered keys
  skip — mirroring real inheritance).
- **Budget**: cap recorded prompts (256) and wall-clock (5s default). Trip → partial
  result, classification `interactive`, transcript retained as the mapped prefix.

Probe result: `{ mode, prompts: [envelope…], switches: [name…], coverage: default-path |
complete | partial, budget_hit? }`. Cached in the archetect cache **keyed by resolved
commit** — tags/commits probe once ever; branches re-probe on the existing freshness
interval; local dirs are never cached.

## 4. Phase 3 — consumers

- `archetect interface <source|catalog-path>`: human-readable table by default;
  `--json` for tooling; `--answers-template` emits a commented YAML answer-file skeleton
  (keys, defaults, options, constraints) — the "instructions for headless interaction, all
  at once" artifact, ready for `-A`.
- MCP `describe { source | path }`: the probe result verbatim. `render`/`catalog_render`
  tool descriptions point at it (they currently point at `interface.yaml` —
  the assessment showed that reference was a dead end for direct sources).
- `catalog_browse` leaf detail swaps its manifest-declared `interface` field (added
  2026-07-22, commit `nkrwoxqsoltn`) for the probe-derived result, lazily, from cache.
- An explicit batch request against an interactive-classified archetype errors, and the
  error names where mapping stopped — the error-as-interface doctrine, extended.

## 5. Phase 4 — drift detection, deprecation

Inventory (2026-07-22): exactly **one** ecosystem archetype ships a sibling
`interface.yaml` (rust-clap-cli-archetype); zero cached manifests carry inline
`interface:`. Blast radius is one self-owned repo. Sequence:

1. `archetect interface --check <source>`: probe, compare against any declared interface,
   report drift (missing keys, type mismatches, phantom declarations). Also run inside
   `archetect check` when the CWD is an archetype.
2. Manifest load warns on `interface:` / `interface.yaml`: "derived interfaces have
   replaced declared ones — delete this after `archetect interface --check` passes."
3. Migrate clap-cli: `validation` → `pattern`, option labels → option tables, `groups` →
   `group` opts. Its interface.yaml is deleted as the proof-of-concept.
4. `InteractionMode` stops being declarable — it is computed (phase 5 hardens it).

## 6. Phase 5 — branch exploration (the prompt graph)

Fork the probe at `select`/`confirm`/`multiselect` decision points, bounded by a
combinatorial budget: explore each option's continuation, merge into a graph
(`prompt → answer → next prompts`). "Generally Q&A with some conditional sections"
terminates quickly; the graph is what a form UI actually wants (show/hide sections on
earlier answers). Classification upgrades: all branches mapped, no budget trip, no dynamic
keys → `batch` (provably one-shot answerable); anything else → `interactive` with the
mapped subgraph. `mode:` in the result is now a fact, not a promise.

Out of scope forever (and fine): archetypes that script whole architectures with
data-driven prompt loops (`model.interactive`). They classify `interactive`; the session
loop is their interface. Returning an error for these in interface mode is correct *only*
when the caller explicitly demanded batch — otherwise return the classification.

## 7. Phase 6 — server + removal

- `DescribeArchetype` RPC in `archetect-core/specs/archetect.proto`, serving cached probe
  results — a web portal queries the server for the form, falls back to the existing
  streaming session for interactive archetypes. Federation forwards describe like browse.
- Remove `interface:` / `interface.yaml` parsing. Load of a manifest still carrying one
  gets a hard error naming this plan's migration steps (matching the "use archetect2"
  major-gate voice). The `interface.rs` types survive as the *derived* interface's shape —
  the schema outlives the hand-written file.

## 8. Acceptance

Prova proofs (`proofs/interface/`, black-box on the shipped binary):

- Flat archetype: probe returns every prompt with full metadata; `--answers-template`
  round-trips (template → `-A` → zero-prompt headless render).
- Conditional archetype: default-path coverage marked; phase-5 graph covers both branches.
- Loop archetype (interactive builder fixture): classifies `interactive`, budget trips,
  prefix retained, no hang.
- Composition: child prompts appear in the parent's transcript; answered keys don't re-ask.
- Switch fixture: `is_enabled` names appear without any switch being set.
- Drift: declared-vs-derived mismatch fixture fails `--check`.
- Probe writes nothing: destination untouched after `archetect interface`.
- MCP: `describe` over stdio returns the same JSON the CLI `--json` emits.

## 9. Risks / open questions

- **Probe executes author Lua without render intent.** Same trust boundary as render, but
  narrower: no writes, no exec, no network modules. Document that `interface` runs the
  script; catalogs/servers probe lazily, not at index time.
- **Nondeterminism** (`now()`, `uuid()`, file reads) can theoretically steer prompts;
  in practice it steers *values*. Cache by commit anyway; `--force-update` re-probes.
- **Answer-sensitive prompt sets** (keys computed from answers) are the honest limit of
  `batch` classification — they classify `interactive`, and that's the right answer.
- **Envelope relocation** (mcp → api) touches the gRPC proto conversions; do it in phase 2
  before consumers multiply.
- ~~Does `group` belong in phase 1, or is script order + `ui` passthrough enough?~~ **Answered
  2026-08-18: neither.** A flat label was too little — it could not say where a group ended or
  nest one inside another, so no renderer ever consumed it. `context:section` is the shape the
  question was reaching for, and `group` was removed in the same change.

<!-- claim: describe-accepts-answers recorded=2026-08-19 -->
Interface derivation should accept answers and derive the REMAINING prompt surface, not the total one. Today `archetect interface` takes no -a/-A and MCP `describe` takes only source/path/explore, so a client cannot ask 'what must I still ask this user?' — only 'what can this archetype be asked?'. Config answers are ignored by the probe too: a key answered in etc.d still comes back as a required prompt. Measured 2026-08-18 against p6m-archetypes: Ybor Studio wants to supply github_default_branch and github_visibility and hide them from the form, and the render session already does exactly that (a supplied answer never surfaces), but the derived interface cannot express it. The same parameter collapses the conditional problem in one stroke — pass messaging=None and messaging_access never appears, pass scm_provider=None and the github pair never appears — with no appears_when predicate vocabulary for each client to reinvent.

<!-- backlog: interface-compatibility-is-detected recorded=2026-08-19 -->
Deriving an interface should FAIL LOUDLY when the archetype is not interface-compatible, so a client can fall back to dynamic rendering instead of being handed a form built from invented values. Today the probe answers unanswered prompts with a placeholder, and any envelope computed from an earlier answer silently resolves against it: p6m-archetypes shipped entity_name default='probe', group_id default='probe', module_path default='github.com/probe/probe' and help text reading '(e.g. probe)' to every client, across 23 of 30 archetypes (measured 2026-08-18, fixed archetype-side by asking those optional and deriving after). Nobody noticed because nothing said anything was wrong. Section 9 already recognises the adjacent case — answer-sensitive prompt SETS classify interactive — this extends it to the prompt ENVELOPE, which currently classifies as fine. A candidate rule: an archetype is interface-compatible if probing it, given whatever answers are supplied, requires inventing no values; that one rule covers both an answer-derived envelope and a conditional branch unreachable without an answer. Worth distinguishing 'fixable, move the derivation after the prompt' from 'genuinely conditional interface', since only the first is an author error.
