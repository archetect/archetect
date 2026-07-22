# rendering — headless renders are the contract

```
archetect render <source> --destination <dir> \
  --headless                # never block on a prompt: unresolved input = ERROR naming the key
  -a service_name=orders    # --answer, repeatable; value parsed as YAML (int/bool/list/map)
  -A answers.yaml           # --answer-file, repeatable (JSON or YAML)
  -s ci -s docker=false     # --switch: add `ci`, remove an inherited `docker`
  -D                        # --use-defaults-all: take the archetype default for the rest
  -d org,team               # --use-default for SPECIFIC keys (repeatable, comma-ok)
  --dry-run                 # show mkdir/write/exec without doing them
```

`<source>` is a git URL (`https://…​.git#v1`, `git@host:org/repo.git`), a local path, or —
via the bare form `archetect <catalog-path>` — an entry in the configured catalog
(`archetect learn catalogs`). Destination defaults to `.`.

## Answer resolution, per prompt key

1. An **answer** for the key (config answers → `-A` files → `-a` flags; last wins) is used.
2. Else if defaults apply (`-D`, or `-d <key>`, or headless mode) → the archetype's default;
   an `optional` prompt yields nil.
3. Else: interactive prompt — or under `--headless`, an ERROR naming the message and key.
   The error IS the interface: answer that key and re-run.

Dotted `-a` keys nest (`-a model.org=acme` → `{ model = { org = "acme" } }`).

## The other levers

| Flag | Effect |
|---|---|
| `-o/--offline` | cache-only: local dirs + already-cached sources; a cold source errors |
| `-U/--force-update` | re-probe every source ref now (branches otherwise re-check on an interval) |
| `-l/--local` | use configured local checkouts instead of clones (`archetect learn sources`) |
| `-e/--allow-exec` | let the archetype run `shell`/`git` commands — off by default; a render that needs it says so |
| `-n/--dry-run` | print every side effect (`[dry-run] write …`) instead of performing it |

Switch overlay semantics are uniform everywhere: a bag of names; `name` adds, `name=false`
removes; layers apply config → catalog entry → CLI, most-specific last.

Exit is non-zero on any script error, unanswered headless prompt, or failed source
resolution — CI keys on it. `archetect check` diagnoses the environment when something is off.

Go deeper: `archetect learn prompts` (the types) · `archetect learn sources` (what `<source>`
accepts) · `archetect learn mcp` (the same render over MCP).
