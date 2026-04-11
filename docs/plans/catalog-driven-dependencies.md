# Catalog-Driven Dependencies — Implementation Plan

## Status

Implementation plan. Drafted 2026-04-10. Companion to
`docs/specs/v3-ecosystem-design.md`. Captures the v3 feature additions
needed before any of the foundational ecosystem repos can exist or be
consumed.

This plan covers Phase 1 of the v3 ecosystem build-out. Once it lands,
Phases 2-9 (building the actual archetypes, libraries, components, and
catalogs) become unblocked.

## What we're building

A small set of changes that together enable the unified single-archetype
model from the ecosystem design:

1. **Catalog entry schema additions** — `library: bool` and `show: bool`
   fields on each catalog entry, both independently controlled.

2. **Library staging at archetype load** — for catalog entries marked
   `library: true`, eagerly resolve the source, stage the resolved
   archetype's `lib/` and `includes/`, and wire them into the consumer's
   runtime.

3. **Multi-includes search path** — `IncludeResolver` accepts a list of
   include directories so library `includes/` dirs and the consumer's
   own `includes/` dir layer cleanly.

4. **Unified `catalog.render(path?, context?)` Lua function** — replaces
   any notion of a separate `component.render`. One function loads a
   catalog entry (lazy unless already eager-staged), runs the default
   render flow recursively, and returns the child's resulting context as
   a *copy*.

5. **Default render flow** — when an archetype is loaded with no script,
   automatically call `catalog.render()` if a catalog exists, otherwise
   print a friendly message and exit 0.

6. **Manifest cleanup** — remove `templating.content` (always
   root-relative), remove `scripting.libraries` and `scripting.modules`
   (replaced by catalog entries with `library: true` and the
   standardized `lib/` directory).

These six changes unblock the entire ecosystem build-out.

## Why this approach

The mechanism is deliberately *thin*. It piggybacks on Lua's existing
`require()` machinery, ATL's existing include resolver, and archetect's
existing source resolution and caching. There is no new template syntax,
no new script DSL, no new file format — just two new flags on catalog
entries and one new Lua function (`catalog.render`).

The single-archetype model (no `type:` field, no `LibraryExports` block,
no enum) means the implementation has fewer cases to enum-match on,
fewer error paths, fewer special variants in the loader. Convention,
not enforcement, guides usage.

## Feature 1: Catalog entry schema additions

### Manifest schema change

The existing `CatalogEntry` parser gains two optional boolean fields:

```rust
// archetect-core/src/manifest.rs (or wherever CatalogEntry lives)

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CatalogEntry {
    pub source: String,                       // existing

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,          // existing

    #[serde(default, skip_serializing_if = "is_default_show")]
    pub show: bool,                           // new — default true

    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub library: bool,                        // new — default false

    // ... existing fields ...
}

fn default_show() -> bool { true }
fn is_default_show(value: &bool) -> bool { *value }
```

`library` and `show` are independent. Setting one does NOT affect the
other. The defaults are:

| Flag | Default | Why |
|------|---------|-----|
| `library` | `false` | Most catalog entries are project archetypes or components, not libraries. Eager pull is expensive; opt-in. |
| `show` | `true` | Most catalog entries are meant to appear in menus. Hidden entries are for "private dependencies" that the script invokes by name. |

### Validation

Minimal — there are no contradictory combinations. All four corners of
`(library, show)` are valid:

| `library` | `show` | Meaning |
|-----------|--------|---------|
| `false` | `true` | Standard catalog entry — appears in menus, lazy-resolved on use |
| `false` | `false` | Private dependency — hidden from menus, lazy-resolved on script use |
| `true` | `true` | Importable library that ALSO appears in menus (rare but valid) |
| `true` | `false` | Importable library, hidden from menus (typical for `*-library` entries) |

The validator does not enforce any combination — author intent is the
only constraint.

### Tests

- `test_catalog_entry_defaults_show_true_library_false`
- `test_catalog_entry_with_library_true`
- `test_catalog_entry_with_show_false`
- `test_catalog_entry_with_both_flags`
- `test_catalog_entry_flags_are_independent`

## Feature 2: Library staging at archetype load

### What "staging" means

When archetect loads an archetype, it walks the catalog entries. For
each entry where `library == true`:

1. **Resolve the source** via the existing source resolver (`source.rs`
   or successor). This fetches a git URL, copies a local path, etc.,
   into the archetype cache.
2. **Read the resolved manifest.** It may itself have a `catalog:`
   section (transitive deps), but for v3.0 we do NOT recursively stage
   transitive `library: true` entries — direct deps only. (This matches
   npm's `dependencies` vs `bundledDependencies` model.)
3. **Determine the resolved archetype's `lib/` and `includes/` paths.**
   These are *always* `<resolved-root>/lib` and `<resolved-root>/includes`,
   no manifest customization.
4. **Add the directories to a `StagedLibrary` record** keyed by the
   consumer-chosen name (the catalog map key).

The result is a `Vec<StagedLibrary>` available to the script registration
code:

```rust
pub struct StagedLibrary {
    pub name: String,                  // map key, e.g. "inflect-helpers"
    pub root: Utf8PathBuf,             // resolved archetype root
    pub lib_dir: Option<Utf8PathBuf>,  // <root>/lib if it exists
    pub includes_dir: Option<Utf8PathBuf>, // <root>/includes if it exists
}
```

Lazy entries (`library: false`) are NOT staged. They are resolved only
when `catalog.render(name)` is called from a script (or when a user
picks the entry from a menu).

### Wiring into the script engine

The existing `register_lua_libraries` function in `modules.rs` is
replaced with `register_staged_libraries`:

```rust
fn register_staged_libraries(
    lua: &Lua,
    staged: &[StagedLibrary],
) -> LuaResult<()> {
    if staged.is_empty() {
        return Ok(());
    }

    let mut prepend_segments = Vec::new();
    for lib in staged {
        let Some(lib_dir) = &lib.lib_dir else { continue };
        // require("<name>.module") → <lib_dir>/module.lua
        // We need a single namespace prefix per library, so the standard
        // package.path approach is to mount each lib's contents under
        // its name in a synthetic-staging dir, OR use a custom searcher.
        // For v3.0 implementation: use the synthetic-staging-dir approach
        // by symlinking each lib_dir into <cache>/staging/<archetype-id>/<name>
        // and adding <staging>/?.lua to package.path.
        prepend_segments.push(format!("{}/?.lua", lib_dir));
        prepend_segments.push(format!("{}/?/init.lua", lib_dir));
    }

    let package: Table = lua.globals().get("package")?;
    let existing: String = package.get("path").unwrap_or_default();
    let new_path = format!("{};{}", prepend_segments.join(";"), existing);
    package.set("path", new_path)?;
    Ok(())
}
```

The `synthetic-staging-dir` approach is preferred over raw `package.path`
because it cleanly namespaces each library under its consumer-chosen
name. Without it, two libraries that both have `casing.lua` would
collide. The staging dir layout looks like:

```
<cache>/staging/<archetype-id>/
    inflect-helpers/    → symlink to <cache>/inflect-helpers-library/lib/
    git-helpers/        → symlink to <cache>/git-helpers-library/lib/
```

Then `package.path` only needs `<cache>/staging/<archetype-id>/?.lua`
and authors write `require("inflect-helpers.casing")`.

On Windows, where symlinks require admin, fall back to copying.

The synthetic staging dir is per-archetype-id (a hash of the consumer's
manifest path or git URL) so concurrent renders of different consumers
don't trample each other.

### Tests

- `test_library_staging_resolves_local_path`
- `test_library_staging_resolves_git_source` (uses a fixture repo)
- `test_library_staging_caches_resolved_source`
- `test_library_staging_creates_namespace_dirs`
- `test_library_staging_skips_lazy_entries`
- `test_consumer_script_can_require_staged_lib`
- `test_two_libraries_with_same_module_name_dont_collide`

## Feature 3: Multi-includes search path

### IncludeResolver change

`IncludeResolver` currently holds a single optional `Utf8PathBuf`. Extend
it to a `Vec<Utf8PathBuf>` of search directories, searched in order.

```rust
pub struct IncludeResolver {
    includes_dirs: Vec<Utf8PathBuf>,  // was: Option<Utf8PathBuf>
    stack: Vec<Utf8PathBuf>,
}

impl IncludeResolver {
    pub fn new(includes_dirs: Vec<Utf8PathBuf>) -> Self { ... }
    pub fn disabled() -> Self {
        Self { includes_dirs: vec![], stack: vec![] }
    }

    fn resolve(&self, relative: &str, line: usize) -> Result<Utf8PathBuf> {
        for dir in &self.includes_dirs {
            let candidate = dir.join(relative);
            if candidate.exists() {
                // existing canonicalize + sandbox check, but against THIS dir
                let canonical = candidate.canonicalize_utf8()?;
                let canonical_root = dir.canonicalize_utf8()?;
                if !canonical.starts_with(&canonical_root) {
                    continue; // sandbox violation, try next
                }
                return Ok(canonical);
            }
        }
        Err(TemplateCompileError::IncludeNotFound { ... })
    }
}
```

Search order: **consumer's own includes dirs first, library includes
dirs after.** This means a consumer can shadow a library include by
placing a file with the same name in its own `includes/`. Useful for
overrides.

Library includes use the namespace prefix in the path:

```
{% include "inflect-helpers/header.atl" %}
```

The consumer's `IncludeResolver` is built with:

```rust
let mut all_dirs = Vec::new();

// Consumer's own includes/ (always at <root>/includes)
let own_includes = archetype.root().join("includes");
if own_includes.exists() {
    all_dirs.push(own_includes);
}

// Each staged library's namespace dir from the synthetic staging.
// The library's includes/ are mounted under <staging>/<name>/includes,
// so the resolver sees `inflect-helpers/header.atl` as
// `<staging>/inflect-helpers/includes/header.atl`.
for lib in staged_libraries {
    if lib.includes_dir.is_some() {
        all_dirs.push(staging_root.join(&lib.name));
    }
}

let resolver = IncludeResolver::new(all_dirs);
```

Wait — this needs more care. With `{% include "inflect-helpers/header.atl" %}`,
the resolver sees the relative path `inflect-helpers/header.atl`. It
needs to find that under `<staging>/inflect-helpers/includes/header.atl`.

There are two ways to do this:

**Option A**: Each library's includes dir is added to the search list
*as itself*, and the include path strips the namespace prefix:

```rust
// includes_dirs:
//   <consumer-root>/includes
//   <cache>/inflect-helpers/includes
//   <cache>/git-helpers/includes

// Author writes: {% include "header.atl" %}
// Resolver finds: <consumer-root>/includes/header.atl OR
//                 <cache>/inflect-helpers/includes/header.atl OR
//                 <cache>/git-helpers/includes/header.atl
```

Problem: namespace collisions. Two libraries with `header.atl` collide.

**Option B**: Each library's includes dir is mounted under its name in
the staging dir, and the include path INCLUDES the namespace:

```rust
// includes_dirs:
//   <consumer-root>/includes
//   <staging>/<archetype-id>/   (which contains symlinks to libraries' includes)

// Staging layout:
//   <staging>/<archetype-id>/inflect-helpers → <cache>/inflect-helpers/includes/
//   <staging>/<archetype-id>/git-helpers     → <cache>/git-helpers/includes/

// Author writes: {% include "inflect-helpers/header.atl" %}
// Resolver finds: <staging>/<archetype-id>/inflect-helpers/header.atl
```

Option B mirrors how the lua side works (namespace prefix in `require`).
Consistent, no collisions, predictable. Adopting Option B.

### Tests

- `test_resolver_finds_in_consumer_includes`
- `test_resolver_finds_in_staged_library_includes`
- `test_resolver_consumer_includes_take_precedence`
- `test_resolver_namespace_prefixed_path`
- `test_resolver_two_libraries_with_same_filename_dont_collide`

## Feature 4: Unified `catalog.render(path?, context?)`

### Lua function signature

```lua
catalog.render(path?, context?)
```

- `path` (optional, string): name of an entry in this archetype's
  catalog. If omitted, presents the catalog menu (filtered by
  `show != false`) and renders whichever entry the user picks.
- `context` (optional, Context userdata): a context to pass to the child.
  The child receives a *copy*. Mutations the child makes are NOT visible
  to the parent. The function returns the child's resulting context as
  a new value.

### Behavior

```lua
-- Pattern 1: assign back (replace via Lua's =)
context = catalog.render("org-prompts", context)

-- Pattern 2: explicit merge
context:merge(catalog.render("project-prompts", context))

-- Pattern 3: sandbox (discard child's mutations)
local sub = catalog.render("preview-tool", context)
-- `context` is unchanged

-- Pattern 4: top-level menu
catalog.render()  -- presents the catalog menu, runs whatever the user picks

-- Pattern 5: name without context (child uses fresh Context.new())
catalog.render("standalone-helper")
```

### Implementation

```rust
// In modules.rs catalog module setup:

catalog_table.set("render", lua.create_function(move |lua, (path, ctx_opt): (Option<String>, Option<AnyUserData>)| {
    // Resolve which catalog entry to render
    let target = match path {
        Some(name) => {
            // Look up the named entry in this archetype's catalog
            let entry = archetype.manifest().catalog()
                .and_then(|c| c.get(&name))
                .ok_or_else(|| LuaError::RuntimeError(
                    format!("catalog.render: no entry named `{}`", name)
                ))?;
            // Resolve the source (lazy fetch, cached)
            resolve_catalog_entry(entry)?
        }
        None => {
            // Browse mode: present a menu of visible entries, get user pick
            let entries: Vec<_> = archetype.manifest().catalog()
                .map(|c| c.iter().filter(|(_, e)| e.show).collect())
                .unwrap_or_default();
            present_menu_and_pick(entries)?
        }
    };

    // Build the child context: copy if provided, fresh if not
    let child_context = match ctx_opt {
        Some(ud) => {
            let parent: std::cell::Ref<Context> = ud.borrow::<Context>()?;
            parent.clone()  // value semantics — child gets a copy
        }
        None => Context::new(archetect.clone(), render_context.clone()),
    };

    // Apply the default render flow to the resolved target
    let result_context = run_default_render_flow(target, child_context)?;

    // Return the child's resulting context as a new userdata value
    Ok(result_context)
})?)?;
```

### `Context::clone()`

This requires `Context` to implement deep clone. It already derives
`Clone` because the underlying `BTreeMap<String, ContextValue>` is
clonable. The only thing to verify: when the child's clone is mutated
(e.g., `ctx:set("foo", "bar")`), the parent's original is unchanged.
Lua userdata for `Context` is a Rust `Context` wrapped — assigning to
the userdata's fields mutates the wrapped value. The `clone()` happens
at the Rust level inside the catalog.render closure, BEFORE the child's
script runs, so we have two distinct `Context` values backing two
distinct userdata wrappers.

### `catalog.render` removes the need for `directory.render`'s parent reference

In the existing v2/v3 model, `directory.render` runs *inside* the parent
script and shares the parent's destination. With `catalog.render`, the
child archetype has its own script which calls its own `directory.render`
with its own context — but the parent's destination is still in scope
because both are using the same `Archetect` instance.

Concretely: the child archetype's `archetype.lua` runs, calls
`directory.render("contents", context)`, and that resolves against the
child archetype's root using the parent's destination from the
RenderContext. The destination isn't copied; only the context is.

### Tests

- `test_catalog_render_with_path_runs_named_child`
- `test_catalog_render_without_path_shows_menu`
- `test_catalog_render_returns_child_context_as_new_value`
- `test_catalog_render_child_mutations_invisible_to_parent`
- `test_catalog_render_assign_back_replaces_parent_context`
- `test_catalog_render_merge_combines_contexts`
- `test_catalog_render_unknown_path_errors_clearly`
- `test_catalog_render_recursive_through_nested_catalog`
- `test_catalog_render_with_no_context_uses_fresh_context`

## Feature 5: Default render flow

### Loader change

When archetect loads an archetype for top-level rendering (`archetect render foo out/`):

```rust
fn render_archetype(archetype: Archetype, ...) -> Result<()> {
    let manifest = archetype.manifest();
    let has_script = archetype.script_path().exists();
    let has_catalog = manifest.catalog().map(|c| !c.is_empty()).unwrap_or(false);

    if has_script {
        // Existing path: run the script, which decides everything
        run_script(archetype, ...)
    } else if has_catalog {
        // No script — implicit catalog.render() at the top level
        let context = Context::new(...);
        run_catalog_menu(archetype, context, ...)
    } else {
        // Neither script nor catalog — friendly message, exit 0
        eprintln!(
            "{} has no script and no catalog —\n\
             it's probably a library, intended for use as a dependency.\n\n\
             To use it from another archetype, declare it in your catalog:\n\n\
             catalog:\n\
               {}:\n\
                 source: {}\n\
                 library: true",
            archetype.name(),
            archetype.name(),
            archetype.original_source(),
        );
        Ok(())
    }
}
```

The `run_catalog_menu` path is the same code that backs `catalog.render()`
when called with no path argument from inside a script.

### Tests

- `test_render_with_script_runs_script`
- `test_render_with_no_script_but_catalog_shows_menu`
- `test_render_with_no_script_no_catalog_friendly_message`
- `test_render_pure_library_friendly_message`

## Feature 6: Manifest cleanup

Three things to remove:

1. **`templating.content`** — gone. `directory.render(path, context)` now
   resolves `path` against `archetype.root()` directly, not against a
   configured content directory.

2. **`scripting.libraries`** — gone. Replaced by catalog entries with
   `library: true`.

3. **`scripting.modules`** — gone. The author's own Lua modules live in
   `<archetype-root>/lib/`, which is the same standardized location used
   by libraries (just on the consumer side instead of an external
   dependency). The script can `require("local-helpers")` and Lua's
   default `package.path` (with `<archetype-root>/lib/?.lua` prepended)
   finds it.

### Migration

Any existing v3 archetype using `templating.content` needs to update.
Run a sed pass:

```bash
# In each .archetype3 directory that uses templating.content
sed -i '' '/^templating:/,/^[a-z]/{ /content:/d; }' archetype.yaml
```

Then audit each `directory.render("foo")` call site and prepend the old
content directory if it wasn't `.`:

```diff
- directory.render("base", context)
+ directory.render("contents/base", context)
```

The audit is straightforward because all `directory.render` calls are
in `archetype.lua` files.

For `scripting.libraries`, the migration is "convert each entry to a
catalog entry with `library: true`":

```diff
- scripting:
-   libraries:
-     - "lib/utils"
-     - "lib/codegen"
+ # The author's own helpers live in lib/, which is on package.path
+ # automatically. No declaration needed.
```

For external library dependencies (none in current v3 archetypes since
the feature didn't fully exist), they become catalog entries:

```yaml
catalog:
  shared-helpers:
    source: git@github.com:example/shared-helpers-library.git
    library: true
```

### Tests

- `test_templating_content_removed_from_manifest_schema`
- `test_directory_render_resolves_against_archetype_root`
- `test_local_lib_dir_on_package_path_by_default`
- `test_legacy_templating_content_field_errors_clearly` (ensures we
  catch unmigrated archetypes)

## Order of work

1. **Feature 6: Manifest cleanup** — removing fields, simplest change,
   gets the schema into its target shape so the rest can build on it.
2. **Feature 1: Catalog entry schema** — adds `library` and `show`,
   small parser change with tests.
3. **Feature 3: Multi-includes search path** — pure resolver refactor,
   no source resolution work yet, lays the groundwork for Feature 2.
4. **Feature 2: Library staging** — depends on 1 (knows about `library: true`)
   and 3 (knows how to add multiple includes dirs). Includes the
   synthetic-staging-dir setup.
5. **Feature 4: `catalog.render`** — depends on 1 (knows about catalog
   entries) and the existing source resolver. Fold in any old
   `component.render` removal here.
6. **Feature 5: Default render flow** — depends on 4 (uses `catalog.render`
   internally for the no-script case).
7. **End-to-end smoke test** — build a tiny library fixture and a
   consumer fixture, verify `require()`, `{% include %}`, and
   `catalog.render` all work in one round-trip.

This order keeps each commit small, isolated, and testable. Total
estimated effort: 1 focused session.

## Risks & open questions

### Symlink fallback on Windows

The synthetic-staging-dir uses symlinks to mount each library's `lib/`
and `includes/` under its consumer-chosen namespace. Windows requires
admin or developer mode for symlinks. Fall back to copying on Windows.
The cost is a small disk-space duplication per archetype run; the
benefit is a consistent runtime layout. Acceptable.

### Tag mutability

A library tagged `0.1.0` can be force-pushed by its maintainer, breaking
every consumer pinned to that tag. Same problem npm has. Mitigations:

- Document strongly: never force-push tags
- Cache by `(source URL, resolved commit hash)` instead of `(source URL, tag)`
- Cache invalidation requires explicit `archetect cache clear` for the
  affected source

The version-aware source resolution spec
(`docs/specs/version-aware-source-resolution.md`) probably covers this
already — verify before implementing.

### Transitive library staging

A library can declare its own `catalog:` block with its own
`library: true` entries. Should the consumer see those transitively
staged in its own runtime? Recommendation: **NO** — direct deps only.
The library author must re-export anything they want consumers to see.
This matches npm's `dependencies` (direct only) vs `bundledDependencies`
(transitive). Reduces namespace pollution and gives library authors
control over their interface.

### Catalog entries with no `lib/` or `includes/`

What happens if an entry is marked `library: true` but the resolved
archetype has neither a `lib/` nor an `includes/` directory? This is a
no-op — nothing gets added to `package.path` or the includes search
list. Probably worth a warning at archetype-load time:
`"warning: catalog entry 'foo' is marked library: true but the resolved
archetype has no lib/ or includes/ directory — nothing to expose"`.
Friendly, not blocking.

### `Context.merge` semantics

The plan introduces `context:merge(other)` as the explicit merge call.
What's the semantics of merge?

Recommendation: **other's keys overwrite self's keys**. Same as `Object.assign`
in JS, `dict.update` in Python, `||=` in Ruby (sort of). The map-like
nature of context makes "later writes win" the natural model.

Should there be a `merge_into(other)` (push self's keys into other)? No
— the existing pattern of `context:merge(catalog.render(...))` is enough.

## Out of scope for Phase 1

These are explicitly not part of this plan:

- **Library version range constraints** (always exact tags for now)
- **Library publishing tooling** (libraries are git repos; the existing
  `git tag && git push --tags` workflow is enough)
- **Library signing / integrity verification** (defer until there's
  evidence of real-world need)
- **A package registry / index** (libraries are discovered via the
  ecosystem catalog, not a centralized registry)
- **Slash-separated catalog paths** (`catalog.render("a/b/c")`) — single
  name only for v3.0
- **Schema-validated context interfaces** (components with declared
  inputs/outputs) — defer until we have real usage patterns

## Verification plan

The plan is complete when these all hold:

1. A library archetype's manifest parses cleanly with no `type:` field
   and no `exports:` block.
2. A consumer archetype with `library: true` on a catalog entry can
   `require("entry-name.module")` to load the library's Lua modules.
3. A consumer archetype can `{% include "entry-name/template.atl" %}`
   to include the library's template partials.
4. A consumer's own `includes/` shadows a library `includes/` for
   same-named files.
5. Two libraries with the same module/template filename do not collide
   because the namespace prefix disambiguates them.
6. `catalog.render("name")` from a script lazy-resolves a non-library
   entry and runs its default render flow.
7. `catalog.render("name", context)` passes a *copy* of the context;
   the parent's context is unchanged after the call returns.
8. `catalog.render()` (no args) presents a menu of visible entries.
9. `archetect render <archetype>` follows the default render flow:
   script → catalog → friendly message.
10. A pure-library archetype rendered standalone produces the friendly
    message and exits 0.
11. The manifest no longer accepts `templating.content`,
    `scripting.libraries`, or `scripting.modules`. Existing v3
    archetypes using these get a clear error pointing to the migration
    path.

The first end-to-end exemplar is `dot-gitignore-archetype` consuming a
`gitignore-fragments-library`. Once that round-trip works, the
mechanism is validated and Phase 2 (foundational component build-out)
can begin.
