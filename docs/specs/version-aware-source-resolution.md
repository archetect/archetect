# Spec: Version-Aware Source Resolution

## Problem

Archetect v2 and v3 coexist during the transition period. Archetypes and catalogs
need to serve both versions simultaneously from the same Git repositories. Users
shouldn't have to manage version-specific refs in their configs.

## Design

When a Git source URL has no explicit ref (no `#tag` or `#branch` suffix),
archetect auto-resolves to the most appropriate version based on its own major
version number.

### Resolution order

Given archetect version `M.x.y` (major version `M`) and a bare URL like
`git@github.com:org/repo.git`:

1. Search for the highest tag matching `vM.*` (e.g., `v3`, `v3.1`, `v3.0.0`)
2. If not found, search downward: `v{M-1}.*`, `v{M-2}.*`, ..., `v1.*`
3. If no versioned tags found, use `main` (or default branch)

### Explicit refs bypass resolution

URLs with an explicit ref are used as-is:

```yaml
# Explicit — no auto-resolution
source: git@github.com:org/repo.git#v1
source: git@github.com:org/repo.git#my-branch
source: git@github.com:org/repo.git#abc123

# Bare — auto-resolution kicks in
source: git@github.com:org/repo.git
```

### Tags vs branches

- **Tags** are the primary discovery mechanism (`v1`, `v2`, `v3`, `v3.1`, etc.)
- **Branches** are used for development and explicit refs, not for auto-resolution
- Tag naming convention: `v{major}` or `v{major}.{minor}` or `v{major}.{minor}.{patch}`

## How this affects each audience

### Catalog/archetype authors

Tag your repo with `v3` when the Lua version is ready. The `v1`/`v2` tags
remain for backwards compatibility. Both versions coexist in the same repo
on different tags/branches.

Migration workflow per archetype:
1. Create `v3` branch, port Rhai → Lua
2. Test locally (local mode uses whatever's checked out)
3. Tag as `v3` when ready
4. Consuming catalogs update their `v3`-tagged catalog.yaml to reference `v3` components
5. Users on archetect3 automatically discover the `v3` versions

### Users

Write bare URLs in configs. Archetect resolves the right version automatically.

```yaml
# v3 default catalog — bare URL, version-aware resolution picks v3.* tags
catalog:
  archetect:
    source: https://github.com/archetect/archetect-catalog.git
```

### Independent archetype authors

Adopting the tagging convention is optional. If an archetype has no versioned
tags, archetect falls back to `main`. The `requires.archetect` field in
`archetype.yaml` provides a second layer of enforcement — even if a v2 binary
somehow resolves a v3-only archetype, the version check rejects it.

## Interaction with existing mechanisms

### `requires.archetect` in archetype.yaml

The manifest requirement check is the enforcement layer. Version-aware
resolution is the discovery layer. Together they provide:

- **Discovery**: archetect3 finds `v3` tags automatically
- **Enforcement**: if discovery fails and falls back to an incompatible version,
  `requires: archetect: "3.0.0"` prevents rendering with the wrong binary

### Local mode

Local overrides resolve the Git URL to a local checkout. The version on disk
(whatever branch is checked out) is used directly. No tag resolution needed —
the developer controls what they're testing.

### Direct path

`archetect3 render /path/to/archetype` uses whatever's on disk. No resolution.

## Initializer implications

Initializers are a special case. They are the bootstrap entry point — the
first archetype a user runs to set up their `~/.archetect/etc/archetect.yaml`.
They must work with whatever binary the user has installed.

**Initializers should stay in Rhai.** Since archetect3 runs Rhai archetypes
perfectly, there is zero cost. Initializers are small scripts (prompt for
org/host, render a config template) that don't benefit from Lua features.
Keeping them in Rhai on the `v1` tag means they work with both v2 and v3
binaries — no version gate needed.

The config template they render can use bare URLs (no `#ref`). archetect3
auto-resolves them; archetect v2 uses the default branch.

Company-specific initializers (e.g., p6m's `archetect-initializer.archetype`)
follow the same principle — stay Rhai, render configs with bare URLs.

This means:
- v2 users are completely unaffected — their initializer and configs don't change
- v3 users run the same Rhai initializer, get auto-resolving configs
- No changes to archetect v2 codebase needed
- No `v3` tag needed for initializers unless Lua-only features are required

## Catalog transition

v2 and v3 master catalogs live in **separate repos** because the top-level
manifest formats diverged too much to cleanly coexist on different tags of
the same repo:

- **v2**: `github.com/archetect/archetect.catalog` (dot-suffixed legacy name,
  v2-only, unchanged)
- **v3**: `github.com/archetect/archetect-catalog` (new repo, v3 unified
  manifest format)

The v3 default config in archetect3 points at the v3 repo. v2 users are
completely unaffected — their catalog stays at the v2 URL.

Individual leaf archetypes (referenced by either catalog) can still coexist
on tags within a single repo. Version-aware resolution then picks the right
tag per client major version. The two-repo split only applies to the master
catalogs themselves.

## Implementation notes

The resolution logic should be implemented in `archetect-core/src/source.rs`
where Git sources are resolved. The current code clones/fetches the repo and
checks out a specific ref. The change:

1. If no ref specified, list remote tags via `git ls-remote --tags`
2. Filter and sort tags matching `v{major}.*` pattern
3. Select highest matching tag for current archetect major version
4. Fall back through lower major versions, then `main`

This can be cached — the tag list for a repo doesn't need to be fetched on
every render if the repo is already cached.
