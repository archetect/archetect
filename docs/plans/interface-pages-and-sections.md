# Pages & Sections — structure for the derived interface

## Why

Archetect has two execution shapes:

1. **Choose-your-own-adventure** — the script asks, the client answers, one prompt at a
   time. Every driver (terminal, MCP, gRPC `StreamingApi`) already speaks it.
2. **Interface-driven** — `archetect interface` / MCP `describe` / gRPC `DescribeArchetype`
   probe the script and hand a client the whole prompt contract up front, so a UI can render
   a form without the script running.

Shape 2 works, and its failure mode is ergonomic rather than technical: **the form gets
long**. A thirty-prompt archetype renders as a thirty-field wall. What a wizard UI (Ybor
Studio) needs is where to break, and what to title each break.

The derived interface already knows *what* is asked and *in what order*. What it cannot know
is the author's intent about **grouping** — that is genuinely new information, and only the
author has it. So the script must be able to say it.

## The shape

Two container verbs, deliberately near-identical:

```lua
local context = Context.new()

context:page("Service Identity", function(ctx)
  ctx:prompt_text("Service Name:", "service_name", { pattern = "^[a-z][a-z0-9-]*$" })

  ctx:section("Ownership", function(ctx)
    ctx:prompt_text("Team:", "team")
    ctx:prompt_text("Email:", "email", { optional = true })
  end)
end)

context:page({ title = "Persistence", key = "persistence_page", help = "Where state lives." }, function(ctx)
  ctx:prompt_select("Database:", "database", { "none", "postgres" })
end)
```

A **page** and a **section** are the same container with a different `kind`. Archetect does
not decide what either means visually — it carries the author's declaration to the renderer,
which decides:

| Renderer | Page | Section |
|---|---|---|
| Ybor Studio (wizard) | a wizard step / screen | a fieldset within the step |
| Terminal (`render`) | a titled banner + rule | a subordinate heading |
| Answers template | a `# ===` block comment | a `# --` block comment |

**Nesting is permissive.** Sections nest in pages, sections nest in sections, and a page
inside a page is not an error. Archetect reports the tree the author built; renderers that
only understand two levels flatten what they don't want. Constraining this would buy nothing
and break archetypes that compose prompts out of shared Lua modules.

### Forms

- `context:page(title, body)` / `context:section(title, body)`
- `context:page(opts, body)` / `context:section(opts, body)` where `opts` is
  `{ title = <string>, key? = <string>, help? = <string>, ui? = <table> }`

`body` receives the context as its first argument (the enclosing `context` upvalue works
identically). Its return values pass through. `title` is required; a missing one is a script
error naming the verb.

`key` is the stable identity a wizard routes on. It defaults to a slug of the title:
lowercased, every run of non-alphanumeric characters collapsed to `_`, ends trimmed
(`"Service Identity"` → `"service_identity"`). Titles are display text and change; keys
should not, so an author who cares pins one.

## The mechanism

Containers are **IO messages**, not script-local bookkeeping — which is what makes them work
identically in every mode:

```rust
ScriptMessage::BeginSegment(SegmentInfo { kind, key, title, help, ui })
ScriptMessage::EndSegment(SegmentEnd { kind, key })
```

Like `Print`/`Display`/`Log*`, they expect no reply. `Context:page` sends `BeginSegment`,
runs the body, and sends `EndSegment` — **including when the body errors**, so the stream is
balanced even on a failed render.

Every consumer then does what it wants with the same stream:

- **`TerminalIoDriver`** prints the title and help as a heading. This is the "segments in the
  CLI" the interface work made obvious: the same declaration that paginates a web wizard
  gives the terminal its section breaks, instead of archetypes hand-rolling
  `output.banner("── Identity ──")`.
- **`ProbeDriver`** records the events alongside the prompt envelopes, and the probe folds
  them into a tree.
- **gRPC** carries them in the `ScriptMessage` oneof, so `archetect connect` and Ybor Studio's
  live session get headings mid-render, not only up front.
- **MCP's session** tracks the stack across turns and stamps each envelope's breadcrumb. It
  is the one interactive surface that must assemble this itself: a gRPC client receives the raw
  Begin/End messages and keeps its own stack, but an MCP agent sees one envelope per tool call,
  and a page opened on turn one is still open on turn four.

## What the derived interface gains

`DerivedInterface` gains `layout` — an ordered tree, always present:

```json
{
  "mode": "batch",
  "prompts": [ { "type": "text", "key": "service_name", ... } ],
  "layout": [
    { "type": "page", "key": "service_identity", "title": "Service Identity",
      "children": [
        { "type": "prompt", "key": "service_name" },
        { "type": "section", "key": "ownership", "title": "Ownership",
          "children": [ { "type": "prompt", "key": "team" } ] }
      ] }
  ]
}
```

- `layout` nodes reference prompts **by key**; `prompts` stays the flat source of truth, so
  nothing is duplicated and nothing can drift.
- An archetype that declares no containers gets a `layout` of bare `prompt` nodes — one code
  path for the client, whether or not the author paginated.
- A page whose prompts are all conditionally skipped still appears (empty `children`), because
  the tree is built from the Begin/End events, not inferred from prompt membership.

`PromptEnvelope` gains `segments` — the breadcrumb of containers a prompt was asked inside,
outermost first, `[{ kind, key, title }]`. Omitted when empty. The tree serves batch
rendering; the breadcrumb serves the **interactive** path, where a client receiving one
envelope at a time can still say "Step 2 of 4 · Ownership".

## Backward compatibility

- Existing archetypes emit no segment messages and are byte-identical in behavior; their
  `layout` is the flat prompt list they already implied.
- The per-prompt `group = "Identity"` opt is **removed**, not kept. Sections superseded it, and
  keeping both would mean two ways to say one thing with one of them a dead end: `group` was a
  string carried to clients with no extent, no nesting, and nothing that rendered it. Passing it
  is now an error naming `context:section` — an error rather than a silent no-op because an
  author who wrote `group` meant to group something, and dropping it quietly ships an ungrouped
  form that looks deliberate. Safe to remove outright: a sweep of the production catalog and the
  materialized cache found zero uses, which is unsurprising for a label no renderer consumed.
- The `ScriptMessage` additions are new proto oneof fields; a client built against the old
  proto sees an unrecognized oneof and skips it, which is exactly the MCP behavior.

## Acceptance

`proofs/interface/pages_and_sections_test.lua`, black-box on the shipped binary:

- The tree: pages, nested sections, permissive nesting, declaration order preserved.
- Metadata: title, help, `ui`, derived key, pinned key.
- `type` distinguishes page from section.
- Breadcrumbs on prompt envelopes.
- A no-container archetype's `layout` is flat prompt nodes; its render is unchanged.
- Containers are inert to rendering: `--headless -D` produces identical files.
- An empty page survives (no prompts inside on the default path).
- A page whose body errors still reports the error (the stream stays balanced).
- The terminal render announces titles and help.
- `archetect connect` shows the same headings over the wire.
- MCP `describe` carries `layout`.
- `--answers-template` groups by container.

## What a wizard actually does

Proven end-to-end: `DescribeArchetype` → walk `layout` to lay out the steps → collect every
answer → render with them. **One describe, one render, zero prompt exchanges** — the streaming
session is an option, not an obligation, and the "N round trips per step" worry does not apply
to a batch-classified archetype at all. An archetype whose prompt set depends on its own
answers classifies `interactive` and is the case still open below.

## Open promises (authored red, not implemented)

- **Page-at-a-time submission, for INTERACTIVE archetypes only** — *held; the need is not
  demonstrated.* The production catalog (`p6m-archetypes3`, 81 archetypes) probes cleanly —
  80 of 81 on the default path, the exception being the catalog entry itself. Forms run median
  2 prompts, max 13. What is not yet measured is how many classify `batch` under `--explore`;
  do that before building this, since the zero-exchange path already covers the `batch` case.

  The look-ahead comes from the **probe**, not from running the page twice. When the script
  blocks on a prompt, the server knows from the probe's layout which page it sits in and sends
  that page's remaining envelopes as one batch; the client answers them together; the server
  feeds the first answer to the blocked prompt and seeds the rest into the context, so the
  later `prompt_*` calls resolve from answers and never reach the wire. **The page body runs
  exactly once.**

  An earlier sketch had the engine execute the body twice — discovery pass, then a real pass
  with answers seeded. Rejected: a page containing `catalog.render` would render its child
  twice, and suppressing writes and exec on the discovery pass does not cover it. An authoring
  surface where adding a page silently doubles what a script does is a footgun no amount of
  documentation pays for.

  Needs a `ClientMessage` variant carrying a keyed answer map, the prompt handler absorbing the
  surplus keys, a per-session probe cache, and a fallback to asking individually when the script
  diverges from the probe.

Flagged `promises = "..."` in the proof suite and listed by `prova owed`.

## Status

Shipped 2026-08-18. 27 proofs in `proofs/interface/pages_and_sections_test.lua` (26 kept,
1 promised). Touches: `archetect-api` (`SegmentInfo`/`SegmentEnd`/`SegmentRef`, two
`ScriptMessage` variants, `PromptEnvelope.segments`), `archetect-core` (`Context:page` /
`Context:section`, `ProbeEvent`, `DerivedInterface.layout`, proto + conversions),
`archetect-terminal-io` (headings in both the driver and the connect client), `archetect-mcp`
(the session's breadcrumb across turns), `archetect-bin` (layout-aware summary and answers
template), plus the LuaCATS stub and `learn prompts`. The per-prompt `group` opt was removed in
the same arc.

<!-- backlog: page-batching-over-the-live-session recorded=2026-08-19 -->
A wizard's Next button costing N round trips is SOLVED for the case that motivated it — not by batching, but by answer-aware derivation. A client describes, renders the step, collects, describes again carrying those answers, and gets back the interface that actually applies; the final render supplies everything at once and asks nothing (proofs/interface/answer_aware_test.lua). That covers conditional archetypes too, because each round resolves the next branch. What is left is narrower and undemonstrated: a client that insists on driving the live StreamingApi still pays one exchange per field. Batching THAT means the server sending a page's remaining envelopes when the script blocks, taking a keyed answer map back, and seeding the surplus into the context — a ClientMessage variant, a per-session probe cache, and a divergence fallback. Note the design constraint if it is ever picked up: the look-ahead must come from the probe, never from re-executing the page body, which an earlier sketch proposed and which would render a catalog.render child twice.
