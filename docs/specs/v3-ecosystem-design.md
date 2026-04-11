# Archetect 3 — Ecosystem Design

## Status

Strategic spec. Drafted 2026-04-10 during the v3 ecosystem build-out
planning. Captures naming, taxonomy, org structure, and the foundational
set of archetypes and libraries to build before v3 ships.

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

1. The naming convention for v3 archetypes, catalogs, and libraries
2. The taxonomy of archetype types we recognize as first-class
3. The org structure (which archetypes live where)
4. The foundational set of archetypes and libraries to build first
5. The phased plan for ecosystem build-out

It is *not* a plan for the v3 binary itself — that lives in the various
phase plans under `docs/plans/`. This is about the *content* that v3
generates, and how that content is organized.

## Naming convention

Three eras of archetect have used three distinct naming schemes:

| Era | Scheme | Example | Status |
|-----|--------|---------|--------|
| v1 | `archetype-NAME` / `catalog-NAME` (prefix) | `archetype-rust-cli` | Archived 2026-04-10 |
| v2 | `NAME.archetype` / `NAME.catalog` (dot suffix) | `rust-cli.archetype` | Frozen, in use |
| v3 | `NAME-archetype` / `NAME-catalog` (hyphen suffix) | `rust-cli-archetype` | New, building out |

The v3 hyphen-suffix convention was chosen because:

- v1's prefix slot is now empty (its repos have been archived)
- The v3 form coexists with v2's dot-suffix form in the same orgs without
  any name collision (different separator, distinguishable by humans and
  by `gh repo list`)
- A hyphenated name reads as a normal repo name — no shell-escape concerns,
  no `.gitignore`-style oddness, no issues with tooling that doesn't
  handle dots in repo names
- When v3 fully takes over, `-archetype` becomes the canonical form
  forever; the dot-suffix form fades out as v2 archetypes are archived

There is no rename event planned in this convention. v2 repos stay where
they are with their original names. v3 repos exist alongside them with
new names. When the v2 binary is renamed `archetect2` and v3 takes over
as `archetect`, the old dot-suffix repos can be archived in place.

### The `-library-archetype` infix

Library archetypes (see taxonomy below) carry an extra `-library-` infix:

```
inflect-helpers-library-archetype
git-helpers-library-archetype
license-headers-library-archetype
```

The infix is visible at a glance in `gh repo list`, signals the type
without needing to look inside the manifest, and reads as a normal
hyphenated name. Component archetypes (the other non-project type) do
*not* get a special infix because they are functionally similar to
project archetypes and the distinction is mostly internal.

## Archetype taxonomy

Five first-class types. Each has a clear purpose and a clear way to
declare itself.

| Type | Purpose | Manifest signal | Example |
|------|---------|-----------------|---------|
| **Project archetype** | Generates a complete deliverable | (default) | `rust-cli-archetype` |
| **Component archetype** | Reusable prompts and sub-renders, invoked by parent | `type: component` | `org-prompts-archetype` |
| **Library archetype** | Shared Lua modules and template partials, no rendering | `type: library` | `inflect-helpers-library-archetype` |
| **Catalog** | Lists and orchestrates archetypes | `catalog:` field set | `rust-catalog` |
| **Tooling** | Not an archetype at all (binaries, actions, docs) | (no manifest) | `archetect-render-action` |

### Project archetype

The default. A project archetype:

- Has prompts that collect parameters from the user
- Renders one or more directories of templates into a destination
- Optionally orchestrates `git`, `archive`, `github`, and other modules
- May depend on component and library archetypes for shared functionality

### Component archetype

A component archetype is invoked by a parent project (or another
component) via `component.render("name", context)`. It:

- Has prompts that contribute to the parent's context
- May render its own templates into the parent's destination
- Acts as a stateful participant in the parent's run, not a passive
  resource

Marker: `type: component` in the manifest. This:

- Prevents the archetype from being rendered standalone (top-level
  invocation produces a clear error)
- Documents intent for catalog tools and IDE features

Examples to build first:

- `org-prompts-archetype` — collects org name, solution name, generates
  case variants and `org-solution-name` derivative
- `project-prompts-archetype` — collects project prefix/suffix, generates
  `project-name` and case variants
- `author-prompts-archetype` — collects author name, email, license choice
- `github-prompts-archetype` — collects repo visibility, default branch,
  remote configuration

### Library archetype

A library archetype is *passive*. It provides Lua modules and template
partials for other archetypes to consume. It has no prompts, no
rendering, no main script — just exports.

Marker: `type: library` in the manifest. This:

- Prevents `directory.render()` and prompt calls from being valid
- Causes archetect to skip the main script entirely (libraries are
  loaded, not executed)
- Causes the manifest validator to require an `exports:` block

Manifest sketch:

```yaml
description: "Inflection and identifier helpers"
authors: ["Jimmie Fulton"]

requires:
  archetect: "3.0.0"

type: library

exports:
  lua: "lib/"           # everything under lib/ becomes require()-able
  includes: "includes/" # everything under includes/ becomes {% include %}-able
```

When another archetype declares this as a dependency, archetect:

1. Resolves the source (git URL or local path), caches it
2. Adds the library's `lib/` directory to Lua's `package.path`
3. Adds the library's `includes/` directory to the template engine's
   includes search path
4. Does *not* execute any script — libraries are loaded, not invoked

Consumer manifest:

```yaml
libraries:
  inflect-helpers:
    source: git@github.com:archetect-common/inflect-helpers-library-archetype.git
    version: "0.1.0"
  license-headers:
    source: git@github.com:archetect-common/license-headers-library-archetype.git
```

Consumer script:

```lua
local inflect = require("inflect-helpers.casing")
local license = require("license-headers")

context:set("license-text", license.apache_2(context:get("author_full")))
```

Consumer template:

```
{% include "license-headers/apache_2.atl" %}
```

### Catalog

A catalog is a manifest with a `catalog:` field listing archetypes,
nested catalogs, and sub-groups. Catalogs may also have prompts and
scripts that prep context before dispatching to a chosen archetype.

Catalogs *do not* take the `type:` field — the presence of `catalog:`
is the signal. A single repo can be both a project archetype and a
catalog (it can be rendered standalone, *or* it can be browsed as a
catalog). The conventional naming is to suffix the repo with `-catalog`
when the catalog dispatch is its primary purpose.

### Tooling

Anything that doesn't fit the four types above. Examples:

- `archetect-3` — the v3 codebase itself
- `archetect-render-action` — GitHub Action wrapping archetect
- `templatize` — CLI for converting existing projects into archetypes
- `homebrew-tap` — Homebrew distribution

These have no manifest, no `-archetype` suffix, and live wherever makes
sense organizationally.

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

| Repo | Purpose |
|------|---------|
| `org-prompts-archetype` | Collects org-name, solution-name, generates `org-solution-name` and case variants |
| `project-prompts-archetype` | Collects project prefix/suffix, generates `project-name` and case variants |
| `author-prompts-archetype` | Collects author name, email, optional license choice |
| `github-prompts-archetype` | Collects repo visibility, default branch, owner |

### Libraries (in `archetect-common/`)

| Repo | Purpose |
|------|---------|
| `git-helpers-library-archetype` | `git init`, `git add`, `git commit`, branch helpers |
| `github-helpers-library-archetype` | `gh repo create`, push helpers, visibility |
| `license-headers-library-archetype` | Apache 2.0, MIT, GPL boilerplate as templates |
| `gitignore-fragments-library-archetype` | Language-specific .gitignore fragments |
| `editor-config-library-archetype` | `.editorconfig`, `.gitattributes` standard contents |
| `github-actions-library-archetype` | Common GitHub Actions workflow patterns |

### Foundational project archetypes (in `archetect-common/`)

| Repo | Purpose |
|------|---------|
| `dot-gitignore-archetype` | Generates a single `.gitignore` file (consumes `gitignore-fragments-library-archetype`) |

The existing v2 `dot-gitignore.archetype` in `archetect-common/` becomes
the v3 model — port it cleanly to use the new library + the new naming.

## Per-language layer

Once the foundational layer is in place, each language ecosystem can be
built incrementally. Pattern (using Rust as the example):

```
archetect-rust/
  rust-prompts-archetype                      (component)
  rust-toolchain-library-archetype            (library: rust-toolchain.toml, rustfmt, clippy config)
  rust-cli-archetype                          (project: simple Clap CLI)
  rust-axum-service-archetype                 (project: HTTP service)
  rust-tonic-service-archetype                (project: gRPC service)
  rust-graphql-service-archetype              (project: GraphQL service)
  rust-workspace-archetype                    (project: multi-crate workspace)
  rust-catalog                                (catalog: lists everything Rust)
```

Each project archetype consumes:

- `org-prompts-archetype` (component, from common)
- `project-prompts-archetype` (component, from common)
- `author-prompts-archetype` (component, from common)
- `rust-prompts-archetype` (component, from rust)
- `git-helpers-library-archetype` (library, from common)
- `license-headers-library-archetype` (library, from common)
- `rust-toolchain-library-archetype` (library, from rust)

The components run in sequence at the top of `archetype.lua`, populate
the context, and then `directory.render()` produces the project. The
libraries are loaded once at script start and used throughout.

## Phased build-out plan

| Phase | Milestone | Effort | Output |
|-------|-----------|--------|--------|
| **0** | Archive v1 repos | 1 cmd | Cleaner orgs |
| **1** | v3 feature: `type: library` + external library resolution + multi-includes search path | 1 session | v3 supports library archetypes |
| **2** | v3 feature: `component.render()` + external component resolution | 1 session | v3 supports component archetypes |
| **3** | Build foundational components in `archetect-common/` | 2-3 sessions | 4 component repos |
| **4** | Build foundational libraries in `archetect-common/` | 2-3 sessions | 6 library repos |
| **5** | Port `dot-gitignore-archetype` as the first end-to-end exemplar | 1 session | Foundation validated |
| **6** | Build `archetect-rust/` from scratch using the foundation | 3-4 sessions | Rust ecosystem complete |
| **7** | Build other language orgs (Java, Go, Python, etc.) | many sessions | Full ecosystem |
| **8** | Build root catalogs (`rust-catalog`, etc.) and master catalog | 1 session | Discoverability |
| **9** | Convert Ybor production archetypes (separate effort) | many sessions | Production ready |
| **10** | Release v3: rename binary, archive v2 repos, redirect docs | 1 session | Cutover |

Phases 1 and 2 are blockers for everything else — they add the v3 features
that the foundational layer relies on. They should land before any
ecosystem repos are created.

## Design principles

These principles inform every decision in the ecosystem build-out:

1. **One purpose per repo.** A library archetype does one thing well.
   A component archetype prompts for one logical group of things. A
   project archetype generates one type of project. Resist the urge to
   bundle.

2. **Composition over duplication.** When two project archetypes share
   functionality, that functionality lives in a library or component,
   not copy-pasted into both.

3. **Leafs before branches.** Build the libraries first, then the
   components, then the project archetypes that consume them. Don't try
   to build a tree top-down.

4. **Versioned dependencies.** Library and component dependencies declare
   a version (tag or branch). Breaking changes in a library bump its
   major version. Consumers pin to the version they were tested against.

5. **Lua-native, not Jinja-flavored.** ATL templates use Lua vocabulary
   (`if x then`, `end`, `local`). Library template partials follow the
   same convention. No two-vocabulary footguns.

6. **Strict mode by default for new archetypes.** New v3 archetypes
   declare `templating.undefined: strict` in their manifest. This catches
   missing context vars at render time instead of producing silent empty
   output. Phase 1 footgun fix in action.

7. **Sourced, not vendored.** Library and component dependencies are
   resolved from git sources at archetype-cache populate time. No vendoring
   into the consumer's repo. The cache is the single source of truth.

8. **Human-friendly errors.** When a library version mismatches, when a
   required component isn't found, when a template fails to render —
   the error message names the archetype, the file, the line, and what
   to do about it. Phase 8.4 of the ATL evolution plan codified this
   for templates; the same standard applies to library/component
   resolution.

## Open questions

These are unresolved and will need decisions before the relevant phase
can start:

1. **Library version constraints.** Do consumers pin to exact tags
   (`version: "0.1.0"`) or accept ranges (`version: "^0.1"`)? Ranges
   need a resolver; tags don't but force coordination on every release.
   Recommendation: start with exact tags, add range support only if it
   becomes painful.

2. **Library circular dependencies.** Should be detected at cache-populate
   time, not at archetype run time. The IncludeResolver already has cycle
   detection for template includes — extend the same pattern to library
   resolution.

3. **Component context isolation.** When a parent calls
   `component.render("org-prompts", context)`, does the component see
   the parent's full context, or just the parent's destination? Most
   v2 archetypes pass the full context, which lets components read
   prior answers but also lets them accidentally clobber state.
   Recommendation: components see a *view* of parent context (read-only
   for keys not in their declared output schema, write for keys in
   their schema). Defer the schema piece until we have real usage.

4. **Library helper discovery.** When a Lua script does
   `require("inflect-helpers.casing")`, how does it know what's
   available? Recommendation: each library exports a `README.md` with
   a Lua API table, and IDE annotations live in `lib/.archetect/` for
   LuaLS-style hover docs. Optional deferred enhancement.

5. **Catalog entry metadata.** Catalogs need to display per-archetype
   summaries in MCP/CLI/IDE listings. The existing
   `archetect.archetype.description()` works for v2 archetypes; v3
   should also expose `tags`, `languages`, `frameworks` from the manifest.
   Already largely supported — needs verification.

## Relationship to other docs

- **`docs/plans/library-archetypes.md`** — implementation plan for the
  v3 features Phase 1 needs (the `type: library` field, external
  resolution, multi-includes path)
- **`docs/plans/atl-engine-evolution.md`** — completed 8-phase plan for
  the ATL templating engine. The ecosystem build-out depends on the
  features delivered there.
- **`docs/plans/archetect-3-clean-break-migration.md`** — the v3
  migration plan for the archetect-3 binary itself.
- **`docs/plans/archetect-3-warts-and-improvements.md`** — the catalog
  of v2 issues that motivated parts of this design.
