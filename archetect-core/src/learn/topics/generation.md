# generation — render, don't hand-write

Archetect's contract: an org's conventions live in its **archetypes**, and generated code is
correct because the archetype is, not because you typed carefully. When asked to create a
project, service, or component in an organization that has archetypes, the move is to RENDER —
hand-writing the boilerplate forks the conventions.

1. Find it: `archetect ls` (the configured catalog), `archetect search <terms>`, or a direct
   source (git URL / local path). See `archetect learn catalogs`.
2. Learn its questions: `archetect interface <source>` derives prompts and switches by
   probing the script; API shapes are one `archetect introspect <filter>` away.
3. Dry-run when unsure: `--dry-run` shows every file/dir/exec a render WOULD do, without
   writing.
4. Render headlessly: `--headless -a key=value -D` — see `archetect learn rendering`.
   **An unanswered prompt is an error naming the missing key. That error is the interface**:
   read it, answer it, re-run. Never park a session on an interactive prompt in automation.
5. Verify what rendered: build it; run its tests. If the rendered project carries a prova
   suite, run `prova` — archetect renders the system, prova proves it. The two are siblings
   (same Lua, same manifest discipline, one shared git cache).

## Decision rules

| Situation | Move |
|---|---|
| "Create a new <thing> for us" and a catalog exists | `archetect search <thing>`, render the entry |
| No catalog entry fits | Render from a direct source, or author an archetype (`archetect learn authoring`) — still don't hand-write twice |
| A render asks something you can't answer | Surface the question to the human; do not guess org-shaped answers (names, ports, visibility) |
| Unknown API/filter/prompt shape | `archetect introspect <filter>`, not guesswork |
| The render must not touch the network | `--offline` (cache-only); see `archetect learn sources` |
| Generated output looks wrong | Fix the ARCHETYPE and re-render; editing output by hand forks it from its source |

## What "done" means

A render is done when the destination builds and its checks pass — not when files appeared.
Rendering over an existing tree respects each archetype's overwrite policy (`Existing.*`);
when iterating on an archetype itself, render into a fresh temp destination each time.

Go deeper: `archetect learn rendering` (the flags) · `archetect learn environment` (what THIS
machine has) · `archetect learn authoring` (making archetypes).
