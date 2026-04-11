# Library Archetypes — Implementation Plan

## Status

Implementation plan. Drafted 2026-04-10. Companion to
`docs/specs/v3-ecosystem-design.md`. Captures the v3 feature additions
needed before any library archetype can exist or be consumed.

This plan covers Phase 1 of the v3 ecosystem build-out. Phase 2
(`component.render()` and external component resolution) is a separate
plan in `docs/plans/component-archetypes.md` (TBW).

## What we're building

Three features that together make library archetypes a first-class
concept:

1. **`type: library` manifest field** — marks an archetype as a
   non-renderable library. Validator forbids prompts and renders;
   compiler skips main script execution.

2. **External library resolution** — `libraries:` in a consumer manifest
   accepts git sources (and local paths) as dependencies. Resolved at
   archetype-cache populate time, cached, and added to the runtime
   `package.path` and includes search path.

3. **Multi-includes search path** — `templating.includes` accepts a list
   of directories instead of a single string. Library `includes/` dirs
   are layered onto this list at runtime so `{% include %}` resolves
   across the consumer's own dir and all of its libraries' dirs.

These three changes unblock the entire ecosystem build-out.

## Why this approach

The mechanism of "drop a directory onto Lua's `package.path` and a list
of search dirs onto the includes resolver" is deliberately *thin*. It
piggybacks on Lua's existing `require()` machinery and ATL's existing
include resolver, both of which already work well in isolation. There
is no new template syntax, no new script DSL, no new file format —
just a way to make the existing mechanisms see content from outside the
archetype's own root.

The alternative — designing a full module system, namespacing scheme,
import statements — was deliberately rejected. v2 had no such concept
and the result was copy-paste duplication everywhere. We don't need a
sophisticated package manager; we need a way to share files cleanly.

## Feature 1: `type: library` manifest field

### Manifest schema change

Add an optional `type` field to `ArchetypeManifest`:

```rust
// archetect-core/src/archetype/archetype_manifest.rs

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ArchetypeType {
    #[default]
    Project,
    Component,
    Library,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ArchetypeManifest {
    description: String,
    // ... existing fields ...

    /// Phase 1 (library archetypes): the kind of archetype this is.
    /// Defaults to `Project` for backwards compatibility with all existing
    /// v2 manifests, which have no `type:` field.
    #[serde(default)]
    archetype_type: ArchetypeType,

    /// What a library exports. Required when `type: library`, forbidden
    /// otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    exports: Option<LibraryExports>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LibraryExports {
    /// Directory whose contents become require()-able. Default: `lib`.
    #[serde(default = "default_lua_exports")]
    pub lua: Utf8PathBuf,

    /// Directory whose contents become {% include %}-able. Default: `includes`.
    #[serde(default = "default_includes_exports")]
    pub includes: Utf8PathBuf,
}

fn default_lua_exports() -> Utf8PathBuf { Utf8PathBuf::from("lib") }
fn default_includes_exports() -> Utf8PathBuf { Utf8PathBuf::from("includes") }
```

### Validation

The manifest validator gains three new rules:

1. If `type: library`, the `exports` block must exist.
2. If `type: library`, `scripting.main` must NOT be present (libraries
   have no main script).
3. If `type: library`, `templating.content` must NOT be set (libraries
   render nothing).

Errors should be specific: "library archetype `foo-library-archetype` is
missing required `exports` block" rather than a generic schema error.

### Runtime enforcement

When archetect loads an archetype:

1. Read manifest
2. If `archetype_type == Library`, mark the archetype as
   non-renderable. The archetype loader returns a `LibraryArchetype`
   variant instead of a `ProjectArchetype`.
3. Top-level commands (`render`) refuse `LibraryArchetype` with a clear
   error: `"foo-library-archetype is a library; it cannot be rendered
   standalone. Add it to your archetype's libraries: declaration."`
4. Library resolution code (Feature 2) accepts only `LibraryArchetype`.

### Tests

- `test_library_manifest_parses_with_exports`
- `test_library_manifest_rejects_missing_exports`
- `test_library_manifest_rejects_main_script`
- `test_library_manifest_rejects_content_directory`
- `test_library_archetype_cannot_be_rendered_standalone`
- `test_project_manifest_with_no_type_field_defaults_to_project`

## Feature 2: External library resolution

### Manifest schema change

The existing `scripting.libraries: Vec<Utf8PathBuf>` becomes a richer
type that accepts either a local path or a git source.

```rust
// archetect-core/src/archetype/archetype_manifest/scripting.rs

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct ScriptingConfig {
    // ... existing fields ...

    #[serde(default)]
    libraries: LinkedHashMap<String, LibraryDependency>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum LibraryDependency {
    /// A local relative path (existing behavior, kept for tests + dev).
    Path(Utf8PathBuf),

    /// A remote source resolved at cache-populate time.
    Source {
        source: String,                    // git URL or path
        #[serde(default)]
        version: Option<String>,           // tag, branch, or commit
    },
}
```

The map key (e.g., `inflect-helpers`) becomes the namespace for
`require("inflect-helpers.casing")`. This is more useful than the
current list-of-paths because:

- Authors can disambiguate libraries with the same module name
- Errors can name the library by its consumer-chosen name
- `require()` paths are stable across library version changes

### Resolution path

A new `LibraryResolver` type sits next to `ArchetypeCache` and `IncludeResolver`:

```rust
// archetect-core/src/library/library_resolver.rs

pub struct LibraryResolver {
    cache_dir: Utf8PathBuf,
    resolved: HashMap<String, ResolvedLibrary>,
}

pub struct ResolvedLibrary {
    pub name: String,
    pub root: Utf8PathBuf,           // local cache dir for this lib
    pub manifest: ArchetypeManifest, // parsed manifest with exports
    pub lua_dir: Utf8PathBuf,        // root + manifest.exports.lua
    pub includes_dir: Utf8PathBuf,   // root + manifest.exports.includes
}

impl LibraryResolver {
    pub fn resolve(&mut self, name: &str, dep: &LibraryDependency) -> Result<&ResolvedLibrary>;
}
```

`resolve()`:

1. If `dep` is `Path`, the root is `archetype_root.join(path)`.
2. If `dep` is `Source`, fetch via the existing source resolver
   (`source.rs`), respecting `version` as branch/tag.
3. Read the cached manifest. Verify `type: library`. Verify exports
   block exists.
4. Construct the `ResolvedLibrary` and store in the map.
5. Return a reference.

Cycle detection: extend `IncludeResolver`'s pattern. A library's
manifest can declare its own `libraries:` block (transitive deps).
The resolver maintains a stack of currently-resolving names; if a
name reappears, raise `LibraryError::CycleDetected`.

### Wiring into the script engine

In `register_lua_libraries` (currently in `modules.rs`), replace the
local-path-only logic with full library resolution:

```rust
fn register_lua_libraries(
    lua: &Lua,
    archetype: &Archetype,
    library_resolver: &mut LibraryResolver,
) -> LuaResult<()> {
    let manifest = archetype.manifest();
    let libraries = manifest.scripting().libraries();
    if libraries.is_empty() {
        return Ok(());
    }

    let mut prepend_segments = Vec::new();
    for (name, dep) in libraries {
        let resolved = library_resolver.resolve(name, dep)
            .map_err(|e| LuaError::RuntimeError(format!(
                "library `{}`: {}", name, e
            )))?;
        // Use the library's namespace as the require() prefix.
        // foo/bar.lua → require("name.foo.bar")
        prepend_segments.push(format!("{}/?.lua", resolved.lua_dir));
        prepend_segments.push(format!("{}/?/init.lua", resolved.lua_dir));
    }

    let package: Table = lua.globals().get("package")?;
    let existing: String = package.get("path").unwrap_or_default();
    let new_path = format!("{};{}", prepend_segments.join(";"), existing);
    package.set("path", new_path)?;
    Ok(())
}
```

### Tests

- `test_library_resolves_local_path`
- `test_library_resolves_git_source` (uses a fixture repo)
- `test_library_caches_on_repeated_resolve`
- `test_library_cycle_detected`
- `test_library_with_wrong_type_rejected`
- `test_library_missing_exports_rejected`
- `test_consumer_script_can_require_library_module`
- `test_consumer_template_can_include_library_partial`

## Feature 3: Multi-includes search path

### Manifest schema change

`templating.includes` becomes a list:

```rust
// archetect-core/src/archetype/archetype_manifest/templating.rs

pub struct TemplatingConfig {
    // ... existing fields ...

    /// Directories that {% include %} resolves paths against. Searched in
    /// order. Default is `["includes"]`. Library archetypes append their
    /// own `includes` directories at runtime.
    #[serde(default = "default_includes_directories",
            deserialize_with = "deserialize_includes_field")]
    includes: Vec<Utf8PathBuf>,
}
```

The custom deserializer accepts both the legacy single-string form and
the new list form for backwards compat:

```rust
fn deserialize_includes_field<'de, D>(d: D) -> Result<Vec<Utf8PathBuf>, D::Error>
where D: serde::Deserializer<'de> {
    use serde::de::Error;
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrList {
        Single(Utf8PathBuf),
        List(Vec<Utf8PathBuf>),
    }
    match StringOrList::deserialize(d)? {
        StringOrList::Single(p) => Ok(vec![p]),
        StringOrList::List(l) => Ok(l),
    }
}
```

### Resolver change

`IncludeResolver` currently holds a single `Option<Utf8PathBuf>`. Extend
it to hold a `Vec<Utf8PathBuf>` of search directories, searched in order.

```rust
pub struct IncludeResolver {
    includes_dirs: Vec<Utf8PathBuf>,  // was: Option<Utf8PathBuf>
    stack: Vec<Utf8PathBuf>,
}

impl IncludeResolver {
    pub fn new(includes_dirs: Vec<Utf8PathBuf>) -> Self { ... }
    pub fn disabled() -> Self { Self { includes_dirs: vec![], ... } }

    fn resolve(&self, relative: &str, line: usize) -> Result<Utf8PathBuf> {
        // For each candidate dir, try to join + canonicalize. First hit wins.
        for dir in &self.includes_dirs {
            let candidate = dir.join(relative);
            if candidate.exists() {
                // ... existing canonicalize + sandbox checks ...
                return Ok(canonical);
            }
        }
        Err(TemplateCompileError::IncludeNotFound { ... })
    }
}
```

The sandbox check (`canonical_file.starts_with(canonical_root)`) runs
against EACH dir, not just one. This means a library include is allowed
if it lives inside that library's includes dir, even though it's outside
the consumer's own includes dir.

### Wiring

When `register_all` builds the `TemplateCache`, it should:

1. Start with the consumer archetype's own `templating.includes` list,
   resolved against the archetype root.
2. Append every resolved library's `includes_dir` from the
   `LibraryResolver`.
3. Pass the combined list to `IncludeResolver::new()`.

```rust
let mut all_includes_dirs = Vec::new();
for dir in templating.includes() {
    let abs = archetype.root().join(dir);
    if abs.exists() {
        all_includes_dirs.push(abs);
    }
}
for resolved in library_resolver.iter() {
    all_includes_dirs.push(resolved.includes_dir.clone());
}
let cache = TemplateCache::new()
    .with_includes_dirs(all_includes_dirs)
    .with_options(opts);
```

### Tests

- `test_includes_field_parses_single_string`  (back-compat)
- `test_includes_field_parses_list`
- `test_resolver_finds_in_first_dir`
- `test_resolver_falls_through_to_second_dir`
- `test_resolver_cycle_per_dir`
- `test_library_include_resolves_via_library_dir`
- `test_consumer_local_includes_take_precedence_over_library`

## Order of work

1. **Feature 1** (manifest changes + validation + runtime enforcement) —
   smallest change, no resolver work, lays the foundation. Merge first.
2. **Feature 3** (multi-includes search path) — pure resolver refactor,
   no source resolution. Merge second.
3. **Feature 2** (external library resolution) — depends on Feature 1
   (knows what `type: library` means) and Feature 3 (knows how to add
   includes dirs). Merge last.

Splitting it this way keeps each commit focused and testable in isolation.

## Risks & open questions

### Library namespace collisions

Two libraries that both export a `casing.lua` will collide in
`package.path`. The map-key namespacing prevents this in practice
(`require("inflect-helpers.casing")` vs `require("git-helpers.casing")`),
but only because we prepend the library's name to its require path.
This needs to be enforced in the wiring code: each library's lua_dir
should be added as `<lua_dir>/?.lua` AND the require path should be
formatted to include the library name as a directory prefix.

Actually a cleaner approach: each library's `lib/` dir is mounted at
its own subdirectory in a *single* synthetic root. Something like:

```
<runtime-staging>/
  inflect-helpers/    → symlink or copy of inflect-helpers/lib/
  git-helpers/        → symlink or copy of git-helpers/lib/
```

Then `package.path` only needs `<runtime-staging>/?.lua` and authors
write `require("inflect-helpers.casing")`. Symlinks are simpler;
on Windows, copies are the fallback. Worth piloting in Feature 2.

### Library tag mutability

A library tagged `0.1.0` can be force-pushed by its maintainer, breaking
every consumer pinned to that tag. This is the same problem npm has with
mutable tags. Mitigations:

- Document strongly: never force-push tags
- Cache by `(source URL, resolved commit hash)` instead of `(source URL, tag)`
- Cache invalidation requires explicit `archetect cache clear` for the
  affected source

The version-aware source resolution spec
(`docs/specs/version-aware-source-resolution.md`) probably covers this
already — verify before implementing.

### Library transitive dependencies

A library can declare its own `libraries:` block, pulling in other
libraries. The resolver needs to handle this recursively. Cycle
detection is the only correctness concern; the rest is just bookkeeping.

Open question: should the consumer see *all* transitive libraries on its
`package.path`, or only its direct dependencies? Direct-only is cleaner
(mental model = npm's `dependencies` vs `bundledDependencies`) but
requires the library author to re-declare any transitive lib they want
their consumers to be able to access. All-transitive is simpler to
implement but pollutes the consumer's namespace. Recommendation: direct
only, force re-declaration. We can revisit if it becomes painful.

## Out of scope for Phase 1

These are explicitly not part of this plan:

- **Component archetype resolution** (Phase 2 — separate plan)
- **Library version range constraints** (always exact tags for now)
- **Library publishing tooling** (libraries are git repos; the existing
  `git tag && git push --tags` workflow is enough)
- **Library signing / integrity verification** (defer until there's
  evidence of real-world need)
- **A package registry / index** (libraries are discovered via the
  ecosystem catalog, not a centralized registry)

## Verification plan

A library archetype is fully working when these all hold:

1. A library archetype with `type: library` parses cleanly.
2. A consumer archetype that declares the library in its `libraries:`
   block can `require()` the library's Lua modules.
3. A consumer archetype can `{% include %}` template partials from the
   library's `includes/` dir.
4. Attempting to render the library standalone produces a clear error.
5. A library that forgets its `exports` block produces a clear error.
6. A circular library dependency produces a clear error.
7. A consumer archetype's local `includes/` dir takes precedence over
   library `includes/` dirs (more specific wins).

The first end-to-end exemplar will be `dot-gitignore-archetype`
consuming a `gitignore-fragments-library-archetype`. Once that round-trip
works, the library mechanism is validated and Phase 3 (foundational
component build-out) can begin.
