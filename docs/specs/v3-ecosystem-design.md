# Archetect 3 — Ecosystem Design

## Status

Strategic spec. Drafted 2026-04-10 during the v3 ecosystem build-out
planning, revised the same day after the catalog-as-universal-dependency
collapse. Captures naming, the single-archetype model, and the
foundational set of repos to build before v3 ships.

## Status snapshot (2026-04-17)

| Area | Status |
|---|---|
| Naming convention (hyphen-suffix: `-archetype`, `-library`, `-catalog`) | shipped (convention codified; `-component` retired in favor of `-library` under the capability model) |
| Single-archetype model (no `type:` field; `library: true` toggles lib/includes staging) | shipped |
| `catalog.render` as universal dispatch | shipped |
| Default render flow (script → catalog → friendly message) | shipped |
| Capability model (prompts / content / context-return / lib exports / include exports) | shipped |
| Foundational libraries (org-prompts, project-prompts, author-prompts, github-prompts, editor-config, license) | shipped locally; GitHub repos pending rename/push |
| Org structure (v3 naming rollout across archetect orgs) | in-progress |
| Phased build-out — Phase 1 (v3 features) | shipped |
| Phased build-out — Phases 2–9 (language catalogs, exemplars, end-to-end) | in-progress / planned |

## Why this document exists

Archetect 3 is more than a refactor of an existing tool. It is a
re-imagining of an entire ecosystem of archetypes that has accumulated
over two prior major versions. The v2 ecosystem grew organically — useful
in places, inconsistent in others, with a handful of patterns that worked
and a handful that should not be carried forward. The Ybor production
catalog is a separate concern with real users; the open-source ecosystem
under the `archetect/`, `archetect-common/`, `archetect-rust/`,
`archetect-writing/`, and related orgs is essentially uninhabited and
gives us free reign to design from principle.

This document is the reference for that re-imagining. It defines:

1. The naming convention for v3 archetypes, libraries, and catalogs
2. The single-archetype model — there is no type system
3. How `catalog.render` works as the universal dispatch
4. The org structure (which archetypes live where)
5. The foundational set of archetypes and libraries to build first
6. The phased plan for ecosystem build-out

It is *not* a plan for the v3 binary itself — that lives in the various
phase plans under `docs/plans/`. This is about the *content* that v3
generates, and how that content is organized.

## Naming convention

Three eras of archetect have used three distinct naming schemes:

| Era | Scheme | Example | Status |
|-----|--------|---------|--------|
| v1 | `archetype-NAME` / `catalog-NAME` (prefix) | `archetype-rust-cli` | Archived 2026-04-10 |
| v2 | `NAME.archetype` / `NAME.catalog` (dot suffix) | `rust-cli.archetype` | Frozen, in use |
| v3 | `NAME-archetype` / `NAME-library` / `NAME-catalog` / `NAME-component` | `rust-cli-archetype` | New, building out |

The v3 hyphen-suffix convention was chosen because:

- v1's prefix slot is now empty (its repos have been archived)
- The v3 form coexists with v2's dot-suffix form in the same orgs without
  any name collision (different separator, distinguishable by humans and
  by `gh repo list`)
- A hyphenated name reads as a normal repo name — no shell-escape concerns,
  no `.gitignore`-style oddness, no issues with tooling that doesn't
  handle dots in repo names
- When v3 fully takes over, the hyphen-suffix forms become canonical
  forever; the dot-suffix form fades out as v2 archetypes are archived

There is no rename event planned. v2 repos stay where they are with
their original names. v3 repos exist alongside them with new names. When
the v2 binary is renamed `archetect2` and v3 takes over as `archetect`,
the old dot-suffix repos can be archived in place.

### The four suffixes are conventions, not types

| Suffix | Convention | Typically contains |
|--------|------------|--------------------|
| `-archetype` | Generates a project | `archetype.lua` + content directory |
| `-library` | Exposes Lua and template partials for reuse | `lib/` and/or `includes/` |
| `-catalog` | Browseable hierarchy of other archetypes | `catalog:` section, no script |
| `-component` | Contributes prompts/context to a parent archetype | `archetype.lua` with prompts, no content |

**These are social signals only.** archetect does not read the suffix or
enforce anything based on it. A repo named `inflect-helpers-library` is
*by convention* a library, but technically nothing stops it from also
having an `archetype.lua` that runs prompts and renders content. A repo
named `rust-cli-archetype` is by convention a project archetype, but
nothing stops it from also exposing a `lib/` directory that other
archetypes consume. Conventions guide; the tool doesn't enforce.

The suffix tells humans what the repo is *intended* for. The contents of
the directory determine what the repo can *actually do*.

## The single-archetype model

There is no `type:` field in the manifest. There is no taxonomy of
"project archetypes" vs "component archetypes" vs "library archetypes".
There is just **archetype**. Every repo with an `archetype.yaml` is an
archetype, period.

What an archetype *can do* depends on what's in its directory:

| Has on disk | Capability |
|-------------|------------|
| `archetype.lua` | Can be executed (drives prompts, rendering, sub-renders) |
| Content directories (any name) | Can be rendered via `directory.render(path, context)` from a script |
| `lib/` | Lua modules become `require()`-able by consumers that pull this archetype as `library: true` |
| `includes/` | Template partials become `{% include %}`-able by consumers that pull this archetype as `library: true` |
| `catalog:` section in manifest | Declares external archetype dependencies and/or browseable children |

These capabilities compose freely. An archetype can have any subset.

### The two standardized directories

`lib/` and `includes/` are **always** at those exact paths. There is no
manifest field to customize them. This is a deliberate choice to remove
a dimension of variability that adds nothing — predictability of
"where do I find this archetype's exports" is more valuable than
flexibility for the few authors who might want a different layout.

Content directories, by contrast, **are** flexible. Authors organize
them however they want — `contents/`, `templates/`, `base/`, or any
other name — and reference them by full root-relative path:

```lua
directory.render("contents/base", context)
if features.includes("monitoring") then
    directory.render("contents/monitoring", context)
end
```

There is no `templating.content` field. The string passed to
`directory.render` is the path from the archetype root. No hidden
prefix.

### What `archetect render <archetype>` does

The default render flow follows a single rule:

```
load the archetype
if it has an archetype.lua:
    run the script (the script decides everything)
else if it has a catalog: section:
    catalog.render()    # show the catalog menu (browse mode)
else:
    print a friendly message and exit 0:
      "<name> has no script and no catalog —
       it's probably a library, intended for use as a dependency"
```

The script always wins when present. A script can do whatever it wants:
prompt the user, render content, call `catalog.render()` to delegate to
a child, call `catalog.render()` at the very end to show a menu of
follow-up archetypes — anything. archetect's job is just to load the
archetype and execute the script.

### `catalog.render` is the universal dispatch

There is one render function for invoking other archetypes:

```lua
catalog.render(path?, context?)
```

- `path` (optional): name of an entry in this archetype's catalog. If
  omitted, presents a menu of all visible (`show != false`) catalog
  entries and renders whichever one the user picks.
- `context` (optional): a `Context` object. The child archetype receives
  a **copy**, not a reference — mutations the child makes to its context
  are *not* visible to the parent.

The function returns the child's resulting context. Parents decide what
to do with it:

```lua
-- Replace via Lua's normal assignment — child's full state becomes ours
context = catalog.render("org-prompts", context)

-- Or merge explicitly into our existing context
context:merge(catalog.render("project-prompts", context))

-- Or sandbox: throw away the child's mutations
local sub = catalog.render("preview-tool", context)
-- `context` is unchanged
```

Replace-via-assign is the natural Lua pattern and the most common case.
Merge is for when the parent wants to take only some of the child's
output. Sandbox-and-discard is for fire-and-forget child renders.

There is no separate `component.render`. Components, libraries, project
archetypes, and catalogs are all *just* archetypes. `catalog.render`
loads whichever one the path resolves to and applies the same default
render flow recursively:

```
catalog.render("org-prompts") →
    look up "org-prompts" in this archetype's catalog →
    resolve its source (lazy) →
    load the resolved archetype →
    apply the default render flow:
        has script    → run it (returns the modified context)
        has catalog   → catalog.render() over its entries
        has neither   → friendly message
```

The path argument is a single name today. Future enhancement may allow
slash-separated paths (`catalog.render("rust-services/rust-cli")`) to
skip intermediate menus.

## Catalog entries

A `catalog:` section is a map from local name → entry definition. Each
entry declares where to fetch the dependency and how it should behave at
load time:

```yaml
catalog:
  inflect-helpers:
    source: git@github.com:archetect-common/inflect-helpers-library.git
    library: true              # eager-resolve at load; expose lib/ and includes/

  org-prompts:
    source: git@github.com:archetect-common/org-prompts-archetype.git
    show: false                # available via catalog.render("org-prompts"),
                               # but hidden from the catalog menu

  rust-cli:
    source: git@github.com:archetect-rust/rust-cli-archetype.git
    description: "Rust CLI with clap"
    # show: true (default), library: false (default)
```

### Entry fields

| Field | Default | Meaning |
|-------|---------|---------|
| `source` | required | Git URL, local path, or other resolvable source |
| `library` | `false` | Eager-resolve at archetype load; add `lib/` to `package.path` and `includes/` to includes search list |
| `show` | `true` | Display in `catalog.render()` menus |
| `description` | (from manifest) | Override for menu display |

`library` and `show` are **independent**. Setting `library: true` does
*not* automatically set `show: false`. If you want a catalog entry to be
both an importable library AND a menu choice, set both `library: true`
and `show: true`. If you want a private dependency that's only used by
the script, set `show: false` and (typically) leave `library: false`.

### How a catalog entry's libraries become available

When a consumer loads an archetype with this in its manifest:

```yaml
catalog:
  inflect-helpers:
    source: git@github.com:archetect-common/inflect-helpers-library.git
    library: true
```

archetect:

1. Resolves the source (caches if not present)
2. Loads the resolved archetype's manifest
3. If the resolved archetype has a `lib/` directory: appends
   `<cache>/inflect-helpers/lib/?.lua` (and `?/init.lua`) to
   `package.path`
4. If the resolved archetype has an `includes/` directory: appends
   `<cache>/inflect-helpers/includes/` to the consumer's include search
   list

The map key (`inflect-helpers`) becomes the namespace prefix:

```lua
local casing = require("inflect-helpers.casing")
```

```
{% include "inflect-helpers/header.atl" %}
```

The map key is the consumer's chosen name. The repo can be renamed
locally:

```yaml
catalog:
  inflect:                   # shorter local alias
    source: git@github.com:archetect-common/inflect-helpers-library.git
    library: true
```

```lua
local casing = require("inflect.casing")
```

This gives consumers rename freedom and prevents namespace collisions
across catalog entries.

### Lazy resolution by default

Catalog entries without `library: true` are **resolved lazily**. They
are not fetched at archetype load. They're only fetched when:

- The script calls `catalog.render("name")`, OR
- The user picks the entry from a `catalog.render()` menu

This means a catalog of 50 entries does not pay 50 git fetches at
startup. It pays for the entries actually used.

`library: true` is the explicit opt-in to eager resolution because the
library's contents need to be on `package.path` *before* the script runs.

## Org structure

The existing GitHub orgs are sufficient. v3 adds repos alongside the v2
ones in the same orgs (no name collision because of the suffix
distinction).

| Org | Purpose |
|-----|---------|
| `archetect/` | Top-level: codebase, master catalogs, language-agnostic primitives, tooling |
| `archetect-common/` | Foundational components and libraries shared across language ecosystems |
| `archetect-rust/` | Rust-specific archetypes, components, libraries, and catalog |
| `archetect-writing/` | Documentation, books, technical writing archetypes |
| `archetect-actions/` | GitHub Actions for CI/CD integration |
| *(future)* `archetect-java/` | Java-specific |
| *(future)* `archetect-go/` | Go-specific |
| *(future)* `archetect-python/` | Python-specific |
| *(future)* `archetect-javascript/` | JavaScript / TypeScript / Node.js |
| *(future)* `archetect-dotnet/` | .NET / C# |

The principle: **one ecosystem per language** in its own org. Each
language ecosystem owns its components, libraries, project archetypes,
and root catalog. Cross-language commonalities live in `archetect-common/`.

## Foundational layer

Before any project archetype can be cleanly built, the foundational
components and libraries must exist. These are the day-1 set, all in
`archetect-common/`.

### Components (in `archetect-common/`)

These are archetypes by convention named with the `-component` suffix.
Each has an `archetype.lua` that collects prompts and returns a
populated context. They have no content directory (or a minimal one).

| Repo | Purpose |
|------|---------|
| `org-prompts-library` | Collects org-name, solution-name, generates `org-solution-name` and case variants |
| `project-prompts-library` | Collects project prefix/suffix, generates `project-name` and case variants |
| `author-prompts-library` | Collects author name, email, optional license choice |
| `github-prompts-library` | Collects repo visibility, default branch, owner |

### Libraries (in `archetect-common/`)

These are archetypes by convention named with the `-library` suffix.
Each has a `lib/` and/or `includes/` directory and (typically) no
script.

| Repo | Purpose |
|------|---------|
| `git-helpers-library` | `git init`, `git add`, `git commit`, branch helpers |
| `github-helpers-library` | `gh repo create`, push helpers, visibility |
| `license-library` | Apache 2.0, MIT, GPL boilerplate as templates |
| `gitignore-fragments-library` | Language-specific .gitignore fragments |
| `editor-config-library` | `.editorconfig`, `.gitattributes` standard contents |
| `github-actions-library` | Common GitHub Actions workflow patterns |

### Foundational project archetypes (in `archetect-common/`)

| Repo | Purpose |
|------|---------|
| `dot-gitignore-archetype` | Generates a single `.gitignore` file (consumes `gitignore-fragments-library` as a `library: true` catalog entry) |

The existing v2 `dot-gitignore.archetype` in `archetect-common/` becomes
the v3 model — port it cleanly to use the new library + the new naming.

## Per-language layer

Once the foundational layer is in place, each language ecosystem can be
built incrementally. Pattern (using Rust as the example):

```
archetect-rust/
  rust-prompts-library                      (component)
  rust-toolchain-library                      (library: rust-toolchain.toml, rustfmt, clippy)
  rust-cli-archetype                          (project: simple Clap CLI)
  rust-axum-service-archetype                 (project: HTTP service)
  rust-tonic-service-archetype                (project: gRPC service)
  rust-graphql-service-archetype              (project: GraphQL service)
  rust-workspace-archetype                    (project: multi-crate workspace)
  rust-catalog                                (catalog: lists everything Rust)
```

Each project archetype's manifest declares the components and libraries
it depends on as catalog entries:

```yaml
catalog:
  org-prompts:
    source: git@github.com:archetect-common/org-prompts-library.git
    show: false
  project-prompts:
    source: git@github.com:archetect-common/project-prompts-library.git
    show: false
  author-prompts:
    source: git@github.com:archetect-common/author-prompts-library.git
    show: false
  rust-prompts:
    source: git@github.com:archetect-rust/rust-prompts-library.git
    show: false
  git-helpers:
    source: git@github.com:archetect-common/git-helpers-library.git
    library: true
  license:
    source: git@github.com:archetect-common/license-library.git
    library: true
  rust-toolchain:
    source: git@github.com:archetect-rust/rust-toolchain-library.git
    library: true
```

The script runs the components in sequence, then renders content using
the libraries' helpers and includes:

```lua
local context = Context.new()
context = catalog.render("org-prompts", context)
context = catalog.render("project-prompts", context)
context = catalog.render("author-prompts", context)
context = catalog.render("rust-prompts", context)

local git = require("git-helpers")
local license = require("license")
context:set("license-text", license.apache_2(context:get("author_full")))

directory.render("contents", context)

if switches.is_enabled("git-init") then
    git.init(context:get("project-name"))
end
```

## Phased build-out plan

| Phase | Milestone | Effort | Output |
|-------|-----------|--------|--------|
| **0** | Archive v1 repos | 1 cmd | Cleaner orgs |
| **1** | v3 features for catalog-driven dependencies (multi-includes path, library/show flags, library staging, unified `catalog.render`, default render flow, `templating.content` removal) | 1 session | v3 supports the unified model |
| **2** | Build foundational components in `archetect-common/` | 2-3 sessions | 4 component repos |
| **3** | Build foundational libraries in `archetect-common/` | 2-3 sessions | 6 library repos |
| **4** | Port `dot-gitignore-archetype` as the first end-to-end exemplar | 1 session | Foundation validated |
| **5** | Build `archetect-rust/` from scratch using the foundation | 3-4 sessions | Rust ecosystem complete |
| **6** | Build other language orgs (Java, Go, Python, etc.) | many sessions | Full ecosystem |
| **7** | Build root catalogs (`rust-catalog`, etc.) and master catalog | 1 session | Discoverability |
| **8** | Convert Ybor production archetypes (separate effort) | many sessions | Production ready |
| **9** | Release v3: rename binary, archive v2 repos, redirect docs | 1 session | Cutover |

Phase 1 is the blocker for everything else — it adds the v3 features
that the foundational layer relies on. It should land before any
ecosystem repos are created.

## Design principles

These principles inform every decision in the ecosystem build-out:

1. **One purpose per repo.** A library does one thing well. A component
   prompts for one logical group of things. A project archetype generates
   one type of project. Resist the urge to bundle.

2. **Composition over duplication.** When two project archetypes share
   functionality, that functionality lives in a library or component as
   a catalog entry, not copy-pasted into both.

3. **Convention, not enforcement.** archetect does not read repo
   suffixes, does not enforce a type system, does not prevent any repo
   from being used in any way another archetype's script calls for. The
   conventions in this document tell humans what's intended; the tool
   trusts the script.

4. **Leafs before branches.** Build the libraries first, then the
   components, then the project archetypes that consume them. Don't try
   to build a tree top-down.

5. **Versioned dependencies.** Catalog entries declare a version (tag,
   branch, or commit). Breaking changes in a library bump its major
   version. Consumers pin to the version they were tested against.

6. **Lua-native, not Jinja-flavored.** ATL templates use Lua vocabulary
   (`if x then`, `end`, `local`). Library template partials follow the
   same convention. No two-vocabulary footguns.

7. **Strict mode by default for new archetypes.** New v3 archetypes
   declare `templating.undefined: strict` in their manifest. This catches
   missing context vars at render time instead of producing silent empty
   output. Phase 1 footgun fix in action.

8. **Sourced, not vendored.** Dependencies are resolved from git sources
   at archetype-cache populate time. No vendoring into the consumer's
   repo. The cache is the single source of truth.

9. **Standardized exports paths.** `lib/` and `includes/` are at fixed
   locations in every archetype that has them. No manifest knob to
   customize. Predictability beats flexibility for these.

10. **Human-friendly errors.** When a library version mismatches, when
    a catalog entry can't be resolved, when a template fails to render —
    the error message names the archetype, the file, the line, and what
    to do about it. Phase 8.4 of the ATL evolution plan codified this
    for templates; the same standard applies to catalog and library
    resolution.

## Open questions

These are unresolved and will need decisions before the relevant phase
can start:

1. **Catalog version constraints.** Do consumers pin to exact tags
   (`version: "0.1.0"`) or accept ranges (`version: "^0.1"`)? Ranges
   need a resolver; tags don't but force coordination on every release.
   Recommendation: start with exact tags, add range support only if it
   becomes painful.

2. **Library circular dependencies.** Should be detected at cache-populate
   time, not at archetype run time. The IncludeResolver already has cycle
   detection for template includes — extend the same pattern to library
   resolution.

3. **Library helper discovery.** When a Lua script does
   `require("inflect-helpers.casing")`, how does it know what's
   available? Recommendation: each library exports a `README.md` with
   a Lua API table, and IDE annotations live somewhere LuaLS can find
   for hover docs. Optional deferred enhancement.

4. **Catalog entry metadata in menus.** Catalogs need to display per-archetype
   summaries in MCP/CLI/IDE listings. The existing
   `archetect.archetype.description()` works for v2 archetypes; v3
   should also expose `tags`, `languages`, `frameworks` from the manifest.
   Already largely supported — needs verification.

5. **Slash-separated catalog paths.** Should `catalog.render("a/b/c")`
   be supported as a way to drill through nested catalogs without
   intermediate menus? Useful for scripted automation. Recommendation:
   single-name only for v3.0; add slash paths if a use case emerges.

## Relationship to other docs

- **`docs/plans/catalog-driven-dependencies.md`** — implementation plan
  for the v3 features Phase 1 needs (catalog entry schema with
  `library`/`show`, multi-includes path, `catalog.render`, default
  render flow, `templating.content` removal)
- **`docs/plans/atl-engine-evolution.md`** — completed 8-phase plan for
  the ATL templating engine. The ecosystem build-out depends on the
  features delivered there.
- **`docs/plans/archetect-3-clean-break-migration.md`** — the v3
  migration plan for the archetect-3 binary itself.
- **`docs/plans/archetect-3-warts-and-improvements.md`** — the catalog
  of v2 issues that motivated parts of this design.
