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
- **MCP's session** ignores them (unknown messages already fall through), so nothing breaks.

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
- The existing per-prompt `group = "Identity"` opt keeps working untouched. It is the flat
  precursor of a section, and promoting it into the tree automatically is deliberately
  **not** done here — see the open spec.
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

## Open promises (authored red, not implemented)

- **`group` promotion**: an archetype using only the flat `group =` opt gets a synthesized
  section tree, so legacy archetypes paginate without an edit.
- **Page-at-a-time submission**: the streaming session accepts a whole page's answers in one
  client message, so a wizard's "Next" is one round trip rather than N.

Both are flagged `promises = "..."` in the proof suite and listed by `prova owed`.

## Status

Shipped 2026-08-18. 23 proofs in `proofs/interface/pages_and_sections_test.lua` (21 kept,
2 promised). Touches: `archetect-api` (`SegmentInfo`/`SegmentEnd`/`SegmentRef`, two
`ScriptMessage` variants, `PromptEnvelope.segments`), `archetect-core` (`Context:page` /
`Context:section`, `ProbeEvent`, `DerivedInterface.layout`, proto + conversions),
`archetect-terminal-io` (headings in both the driver and the connect client), `archetect-bin`
(layout-aware summary and answers template), plus the LuaCATS stub and `learn prompts`.
