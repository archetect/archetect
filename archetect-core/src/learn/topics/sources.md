# sources — addressing, caching, and the offline story

A `<source>` (CLI arg or catalog `source:`) is one of:

```
https://github.com/acme/thing.git#v1     # git URL; #tag / #branch / #commit ref
git@github.com:acme/thing.git            # SSH shorthand
./relative/or/absolute/path              # local dir (catalog-relative when in a catalog)
```

No ref → the default branch. **Only a bare commit is immutable** — cached once, never
re-probed. Everything symbolic (branches AND tags) is mutable and re-checked when the update
interval lapses (hash-gated: one `ls-remote`, no change → no re-clone). Tags are mutable on
purpose: the floating-major convention (`v1` tracking the latest v1.x.y) depends on it.

The interval is the whole story for freshness. Within it there is **zero network** — a tag
that moved upstream keeps serving the cached tree until the TTL lapses, which is correct but
surprising right after you publish. Two ways out:

```
archetect render <source> -U         # force a re-probe now, one run
```

```yaml
# archetect.yaml — a shorter TTL, e.g. while authoring libraries
updates:
  interval: 60        # seconds; default 86400 (1 day)
```

## The cache (shared with prova — same trees, both binaries)

[[slot:cache_state]]

Content-addressed by COMMIT: each resolved ref materializes an immutable per-commit tree,
leased (shared flock) for the duration of a render, reaped by retention when unused. Two
concurrent renders of one source never conflict.

| Verb | Effect |
|---|---|
| `archetect cache pull [source]` | recursively fetch everything reachable (warm CI/offline) |
| `archetect cache invalidate [source]` | force re-fetch on next use |
| `archetect cache prune` | reap unleased trees past retention |
| `archetect cache clear` | delete the whole cache |

## Locals — dev mode for archetype authors

[[slot:locals]]

With locals configured, a source whose repo directory name exists under a locals path uses
that CHECKOUT instead of a clone — edit an archetype and re-render a consumer without
pushing. `-l/--local` toggles per run.

## Offline and freshness, per run

- `-o/--offline` — cache-only; a source that was never cached is an error, never a hang.
- `-U/--force-update` — re-probe every ref now.
- Neither changes WHAT renders — a cached tree is byte-identical to its commit.

Go deeper: `archetect learn catalogs` (where sources are declared) · `archetect learn
environment` (this machine's cache and locals, computed).
