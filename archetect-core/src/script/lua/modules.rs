use std::cell::RefCell;
use std::rc::Rc;

use mlua::{AnyUserData, Error as LuaError, Lua, Result as LuaResult, Table, Value};

use crate::archetype::archetype::{Archetype, OverwritePolicy};
use crate::archetype::render_context::RenderContext;
use crate::Archetect;

use super::context::Context;
use crate::templating::atl::render::{self as lua_render, TemplateCache};

pub fn register_all(
    lua: &Lua,
    archetype: &Archetype,
    archetect: &Archetect,
    render_context: &RenderContext,
) -> LuaResult<()> {
    register_context_constructor(lua, archetect, render_context)?;
    super::cases::register_cases(lua)?;
    register_existing_constants(lua)?;
    register_location_constants(lua)?;
    register_archetect_module(lua, archetect)?;

    // Phase 1, commit 4: stage any catalog entries marked `library: true`
    // before wiring Lua paths. The stager resolves the source via the
    // existing source layer (git/local/cached), then symlinks (or copies
    // on Windows) the resolved archetype's lib/ and includes/ into a
    // synthetic per-consumer staging dir under the archetect cache.
    let staged_libraries = if let Some(catalog) = archetype.manifest().catalog() {
        let mut stager = crate::library::LibraryStager::new(archetect.clone(), archetype.root());
        stager.stage(catalog).map_err(|e| {
            LuaError::RuntimeError(format!("library staging failed: {}", e))
        })?
    } else {
        Vec::new()
    };

    // archetype.* needs the staging registry to back mount_key() / is_library() /
    // is_standalone(), which match the calling chunk's source path against
    // staged library dirs. Registered after staging so the registry is final.
    register_archetype_module(lua, archetype, render_context, &staged_libraries)?;

    register_lua_libraries(lua, archetype, &staged_libraries)?;

    let filters = create_builtin_filters(lua)?;
    // Build the template cache with manifest-driven configuration:
    //   - `templating.undefined: strict | lenient` for variable resolution
    //   - `templating.trim_blocks` / `lstrip_blocks` for whitespace controls
    //   - includes search list: consumer's own <root>/includes first, then
    //     each staged library's includes/ namespace dir
    use crate::archetype::archetype_manifest::templating::UndefinedMode;
    use crate::templating::atl::CompileOptions;
    let templating = archetype.manifest().templating();

    use crate::templating::atl::include_resolver::IncludeTrust;
    let mut includes_dirs: Vec<(camino::Utf8PathBuf, IncludeTrust)> = Vec::new();
    // Consumer's own includes/ wins over library includes (more specific).
    // User trust: this is content the archetype author put in their own
    // tree, so the sandbox check enforces no symlink escape.
    let local_includes = archetype.root().join("includes");
    if local_includes.exists() {
        includes_dirs.push((local_includes, IncludeTrust::User));
    }
    // Each staged library contributes its parent dir (the namespace mount
    // point), so `{% include "lib-name/file.atl" %}` resolves under it.
    // We add the SHARED parent <staging>/includes/ once if any library
    // staged an includes/ — all libraries' staged includes live as
    // siblings inside it, so a single search root catches all of them.
    //
    // System trust: archetect built these symlinks itself, pointing at
    // known library sources. The sandbox check is intentionally bypassed
    // because the canonicalized files live in the libraries' source dirs,
    // not under the staging root.
    if let Some(first_with_includes) = staged_libraries
        .iter()
        .find_map(|lib| lib.includes_dir.as_ref())
    {
        if let Some(parent) = first_with_includes.parent() {
            includes_dirs.push((parent.to_owned(), IncludeTrust::System));
        }
    }

    let cache = TemplateCache::new()
        .with_options(CompileOptions {
            strict: matches!(templating.undefined(), UndefinedMode::Strict),
            trim_blocks: templating.trim_blocks(),
            lstrip_blocks: templating.lstrip_blocks(),
        })
        .with_includes_dirs(includes_dirs);
    let cache = Rc::new(RefCell::new(cache));
    register_lua_directory_module(lua, archetype, archetect, render_context, &filters, cache.clone())?;
    register_lua_file_module(lua, archetype, archetect, render_context, &filters, cache)?;
    register_lua_template_module(lua, archetect, render_context, &filters)?;

    register_catalog_module(lua, archetype, archetect, render_context)?;
    register_format_module(lua)?;
    register_exit(lua)?;
    register_log(lua, archetect)?;
    register_output(lua, archetect)?;
    Ok(())
}

/// Wire `package.path` so the script can `require()` Lua modules from:
///
/// 0. The archetype root itself — enables `require("lib")` to resolve to
///    `<root>/lib/init.lua` for libraries that follow the
///    `lib/init.lua` main-module convention. Also makes any top-level
///    directory with an `init.lua` requirable by its directory name.
///    In practice only `lib/` ships with an `init.lua`, so the only
///    observable effect is the self-require convenience.
///
///    **Self-only scope:** this entry applies to the currently-rendering
///    archetype's own script execution. It does NOT extend to consumers
///    that mount this archetype with `library: true` — staging only
///    symlinks `lib/` under the map-key, so sibling top-level directories
///    in a library (`prompts/`, `providers/`, etc.) stay unreachable
///    from the consumer side. Library authors should keep internal
///    modules under `lib/` and cross-reference them via
///    `require("lib.<submodule>")` or the `local mod_name = ...`
///    varargs trick so the prefix adapts to whatever name the caller
///    used. See `docs/plans/self-requirable-lib.md`.
/// 1. The consumer's own `<root>/lib/` directory — implicit, no declaration
///    needed. Authors of project archetypes drop helpers into `lib/` and
///    `require("helpers")` from their main script.
/// 2. Each staged library's mounted lib directory, namespaced under the
///    consumer-chosen catalog map key. So `require("inflect-helpers.casing")`
///    resolves to `<staging>/lib/inflect-helpers/casing.lua` because the
///    library's own `lib/` was symlinked under that namespace.
///
/// All entries are prepended to `package.path` in order, so they take
/// precedence over Lua's default search paths.
fn register_lua_libraries(
    lua: &Lua,
    archetype: &Archetype,
    staged_libraries: &[crate::library::StagedLibrary],
) -> LuaResult<()> {
    let mut prepend_segments: Vec<String> = Vec::new();

    // 0. Archetype root — lets a library's own shim reach its main
    //    module at `lib/init.lua` via `require("lib")`. The `?.lua`
    //    entry here is redundant with (1) for the lib/ subdirectory
    //    but harmless; `?/init.lua` is the one that does the real
    //    work for self-require.
    let root = archetype.root();
    prepend_segments.push(format!("{}/?.lua", root));
    prepend_segments.push(format!("{}/?/init.lua", root));

    // 1. Consumer's own lib/ — implicit local helpers.
    let local_lib = archetype.root().join("lib");
    if local_lib.exists() {
        prepend_segments.push(format!("{}/?.lua", local_lib));
        prepend_segments.push(format!("{}/?/init.lua", local_lib));
    }

    // 2. Staged library lib dirs share a common parent
    //    (<staging>/lib/), and each library's lib/ is symlinked underneath
    //    its catalog map key. We add the SHARED parent so a single
    //    package.path entry covers every library.
    if let Some(first_with_lib) = staged_libraries
        .iter()
        .find_map(|lib| lib.lib_dir.as_ref())
    {
        if let Some(parent) = first_with_lib.parent() {
            prepend_segments.push(format!("{}/?.lua", parent));
            prepend_segments.push(format!("{}/?/init.lua", parent));
        }
    }

    if prepend_segments.is_empty() {
        return Ok(());
    }

    let package: Table = lua.globals().get("package")?;
    let existing: String = package.get("path").unwrap_or_default();
    let new_path = if existing.is_empty() {
        prepend_segments.join(";")
    } else {
        format!("{};{}", prepend_segments.join(";"), existing)
    };
    package.set("path", new_path)?;
    Ok(())
}

fn register_context_constructor(
    lua: &Lua,
    archetect: &Archetect,
    render_context: &RenderContext,
) -> LuaResult<()> {
    let archetect = archetect.clone();
    let render_context = render_context.clone();
    let context_table = lua.create_table()?;
    context_table.set(
        "new",
        lua.create_function(move |_, ()| {
            Ok(Context::new(archetect.clone(), render_context.clone()))
        })?,
    )?;
    lua.globals().set("Context", context_table)?;
    Ok(())
}

// ── archetect module (binary introspection + answers) ───────────────

fn register_archetect_module(
    lua: &Lua,
    archetect: &Archetect,
) -> LuaResult<()> {
    let archetect_table = lua.create_table()?;

    // `archetect` is the *binary / process / platform* — info that is
    // identical across consecutive invocations. Per-invocation state
    // (switches, answers) lives on the `archetype` global instead.

    let version = archetect.version().clone();
    archetect_table.set("version", version.to_string())?;
    archetect_table.set("version_major", version.major as i64)?;
    archetect_table.set("version_minor", version.minor as i64)?;
    archetect_table.set("version_patch", version.patch as i64)?;

    // Process-mode flags (folded in from the old top-level `runtime` global).
    archetect_table.set("is_offline", archetect.is_offline())?;
    archetect_table.set("is_headless", archetect.is_headless())?;
    archetect_table.set(
        "locals_enabled",
        archetect.configuration().locals().enabled(),
    )?;

    // archetect.env — platform info (os, arch, family, is_* booleans).
    archetect_table.set("env", build_env_table(lua)?)?;

    lua.globals().set("archetect", archetect_table)?;
    Ok(())
}

// ── archetype module (self-inspection of current archetype) ─────────

fn register_archetype_module(
    lua: &Lua,
    archetype: &Archetype,
    render_context: &RenderContext,
    staged_libraries: &[crate::library::StagedLibrary],
) -> LuaResult<()> {
    let archetype_table = lua.create_table()?;

    // `archetype` is the *currently-rendering archetype*: manifest
    // metadata + the parameters that were supplied for THIS render.
    // Anything that would change between two consecutive invocations
    // belongs here. (Binary version / platform / process flags live
    // on the `archetect` global.)

    archetype_table.set(
        "description",
        archetype.directory().manifest().description().to_string(),
    )?;
    archetype_table.set("directory", archetype.directory().root().to_string())?;
    // archetype.destination — absolute path where files are being rendered.
    // Tracks the `-d` flag. Pair with `Location.Destination` when using
    // `file.*` operations; use the string directly when passing to shell
    // helpers or constructing paths by hand.
    archetype_table.set(
        "destination",
        render_context.destination().to_string(),
    )?;

    let authors: Vec<String> = archetype
        .directory()
        .manifest()
        .authors()
        .iter()
        .map(|a| a.to_owned())
        .collect();
    archetype_table.set("authors", authors)?;

    // archetype.switches — switches supplied to this invocation.
    archetype_table.set("switches", build_switches_table(lua, render_context)?)?;

    // archetype.answers() — fresh table of the pre-supplied answers.
    let answers = render_context.answers().clone();
    archetype_table.set(
        "answers",
        lua.create_function(move |lua, ()| context_map_to_lua_table(lua, &answers))?,
    )?;

    // archetype.mount_key() — when called from inside a staged library
    // (one mounted by the parent's catalog with `library: true`), returns
    // the catalog map-key under which it was mounted. Used by libraries
    // that publish include paths or other strings that depend on the
    // parent-chosen namespace. Returns nil from the parent's main script
    // and from a library running via its own standalone shim.
    //
    // Detection works by matching the calling chunk's source path
    // (debug.getinfo) against the staged library directories captured
    // at registration time.
    //
    // archetype.is_library() / archetype.is_standalone() are convenience
    // booleans over the same logic.
    let registry: Vec<(String, String)> = staged_libraries
        .iter()
        .filter_map(|lib| {
            lib.lib_dir
                .as_ref()
                .map(|d| (d.to_string(), lib.name.clone()))
        })
        .collect();

    let registry_for_mount = registry.clone();
    archetype_table.set(
        "mount_key",
        lua.create_function(move |lua, ()| -> LuaResult<Value> {
            match caller_mount_key(lua, &registry_for_mount)? {
                Some(name) => Ok(Value::String(lua.create_string(&name)?)),
                None => Ok(Value::Nil),
            }
        })?,
    )?;

    let registry_for_lib = registry.clone();
    archetype_table.set(
        "is_library",
        lua.create_function(move |lua, ()| -> LuaResult<bool> {
            Ok(caller_mount_key(lua, &registry_for_lib)?.is_some())
        })?,
    )?;

    let registry_for_standalone = registry.clone();
    archetype_table.set(
        "is_standalone",
        lua.create_function(move |lua, ()| -> LuaResult<bool> {
            Ok(caller_mount_key(lua, &registry_for_standalone)?.is_none())
        })?,
    )?;

    // archetype.include_path(rel) — sugar over mount_key() for libraries
    // publishing include paths into the parent's template world.
    //
    // Library mode: returns "<MOUNT_KEY>/<rel>" so the parent can use the
    // value directly in `{% include %}`.
    // Standalone: returns rel unchanged. The library's own includes/ is
    // already on the include search path under no prefix in standalone
    // runs, so an unprefixed path resolves correctly there too.
    //
    // Saves library authors from doing the `mount_key() or "<conventional>"`
    // dance for the common case.
    let registry_for_path = registry;
    archetype_table.set(
        "include_path",
        lua.create_function(move |lua, rel: String| -> LuaResult<Value> {
            let resolved = match caller_mount_key(lua, &registry_for_path)? {
                Some(name) => format!("{}/{}", name, rel),
                None => rel,
            };
            Ok(Value::String(lua.create_string(&resolved)?))
        })?,
    )?;

    lua.globals().set("archetype", archetype_table)?;
    Ok(())
}

/// Look up the mount key for whichever staged library the calling Lua
/// chunk lives in, by matching the Lua call stack frame above this Rust
/// callback against the staged-library lib directories.
///
/// Stack level 1 is the Lua function that invoked
/// archetype.mount_key/is_library/is_standalone — i.e. the call site we
/// want to identify. Lua reports file-loaded chunk sources as
/// `@<absolute-path>`; we strip the `@` and prefix-match against each
/// staged library's lib_dir. A match requires the next character to be a
/// path separator (or end of string) so e.g. `/staging/lib/foo-bar/x.lua`
/// doesn't match a staged `foo` dir.
///
/// We use mlua's `Lua::inspect_stack` (Rust-side wrapper over
/// `lua_getstack` + `lua_getinfo`) rather than reaching for the `debug`
/// library at the Lua level — mlua's safe `Lua::new()` ships without
/// `debug` loaded as a security default, but the C-level introspection
/// is always available.
fn caller_mount_key(lua: &Lua, registry: &[(String, String)]) -> LuaResult<Option<String>> {
    if registry.is_empty() {
        return Ok(None);
    }
    let source: Option<String> = lua.inspect_stack(1, |dbg| {
        dbg.source()
            .source
            .as_ref()
            .map(|s| s.as_ref().to_string())
    }).flatten();
    let Some(source) = source else {
        return Ok(None);
    };
    let path = source.strip_prefix('@').unwrap_or(&source);

    for (lib_dir, name) in registry {
        if path.starts_with(lib_dir.as_str()) {
            let after = &path[lib_dir.len()..];
            if after.is_empty() || after.starts_with('/') || after.starts_with('\\') {
                return Ok(Some(name.clone()));
            }
        }
    }
    Ok(None)
}

// ── catalog module (render catalog entries by path) ─────────────────

fn register_catalog_module(
    lua: &Lua,
    archetype: &Archetype,
    archetect: &Archetect,
    render_context: &RenderContext,
) -> LuaResult<()> {
    let catalog_table = lua.create_table()?;

    // catalog.render(path?, context, opts?)
    //   - catalog.render(context)                  → present root catalog entries
    //   - catalog.render("services", context)      → present "services" group as menu
    //   - catalog.render("services/grpc", context) → render the leaf archetype directly
    let parent = archetype.clone();
    let arc = archetect.clone();
    let ctx = render_context.clone();

    catalog_table.set(
        "render",
        lua.create_function(
            move |_, args: mlua::MultiValue| {
                let (path, context_ud, opts) = parse_catalog_render_args(&args)?;

                let raw_catalog = parent.manifest().catalog().ok_or_else(|| {
                    LuaError::RuntimeError("No catalog entries defined in archetype.yaml".to_string())
                })?;

                // Normalize relative source paths against the consumer
                // archetype's root before dispatching. This makes
                // catalog entries portable across the directories
                // archetect might be invoked from. Library staging in
                // commit 4 already does this; mirror it here so the
                // dispatch path has the same semantics.
                let catalog = normalize_catalog_sources(parent.root(), raw_catalog);
                let catalog = &catalog;

                let context_map = {
                    let context = context_ud.borrow::<Context>()?;
                    context.to_context_map()
                };

                let mut destination = ctx.destination().to_path_buf();
                if let Some(ref opts) = opts {
                    if let Ok(dest_str) = opts.get::<String>("destination".to_string()) {
                        let dest_str = restrict_path(&dest_str)?;
                        destination = destination.join(dest_str);
                    }
                }

                let mut child_context = RenderContext::new(destination, context_map);

                // Apply opts-level overrides (these override catalog entry defaults
                // for switches/defaults). The dispatch module applies entry-level
                // values first, so we apply opts AFTER the dispatch by leveraging
                // the fact that for opts to take effect on a leaf render we need
                // to set them on the context BEFORE calling dispatch.
                if let Some(ref opts) = opts {
                    if let Ok(switches) = opts.get::<Vec<String>>("switches".to_string()) {
                        child_context.set_switches(switches.into_iter().collect());
                    }
                    if let Ok(defaults) = opts.get::<Vec<String>>("use_defaults".to_string()) {
                        child_context.set_use_defaults(defaults.into_iter().collect());
                    }
                    if let Ok(use_defaults_all) = opts.get::<bool>("use_defaults_all".to_string()) {
                        child_context.set_use_defaults_all(use_defaults_all);
                    }
                }

                let result = crate::catalog::dispatch::dispatch(&arc, catalog, path.as_deref(), child_context)
                    .map_err(|e| match e {
                        // Preserve the "Prompt aborted" sentinel so the
                        // outer execute() detects the cancel via
                        // is_prompt_abort and propagates it up as a
                        // clean exit rather than a generic error.
                        crate::errors::ArchetectError::ArchetypeError(
                            crate::errors::ArchetypeError::PromptAborted,
                        ) => LuaError::RuntimeError("Prompt aborted".to_string()),
                        other => LuaError::RuntimeError(format!("Catalog error: {}", other)),
                    })?;

                // Convert the child's resulting ContextValue back into a fresh
                // Lua Context userdata. The parent's `context` argument is
                // unchanged — value semantics — and the parent decides what
                // to do with the returned value via Lua's normal assignment
                // (`context = catalog.render(...)`) or by calling
                // `context:merge(catalog.render(...))`.
                child_value_to_context(&arc, &ctx, result)
            },
        )?,
    )?;

    lua.globals().set("catalog", catalog_table)?;
    Ok(())
}

/// Walk a catalog and rewrite each entry's `source:` against the consumer
/// archetype's root, recursing into nested catalog groups. Returns a new
/// `LinkedHashMap` so the original manifest is unchanged.
fn normalize_catalog_sources(
    consumer_root: &camino::Utf8Path,
    catalog: &linked_hash_map::LinkedHashMap<String, crate::manifest::CatalogEntry>,
) -> linked_hash_map::LinkedHashMap<String, crate::manifest::CatalogEntry> {
    let mut out = linked_hash_map::LinkedHashMap::new();
    for (name, entry) in catalog {
        let mut cloned = entry.clone();
        if let Some(ref src) = cloned.source {
            cloned.source = Some(crate::library::normalize_source(consumer_root, src));
        }
        if let Some(ref nested) = cloned.catalog {
            cloned.catalog = Some(normalize_catalog_sources(consumer_root, nested));
        }
        out.insert(name.clone(), cloned);
    }
    out
}

/// Convert a child archetype's returned `ContextValue` into a freshly
/// constructed Lua `Context` userdata. The new Context is independent of
/// the parent's — assigning the returned value back to the parent's
/// variable replaces it; calling `parent:merge(child)` combines them.
fn child_value_to_context(
    archetect: &Archetect,
    render_context: &RenderContext,
    value: archetect_api::ContextValue,
) -> LuaResult<Context> {
    use archetect_api::ContextMap;
    let map: ContextMap = match value {
        archetect_api::ContextValue::Map(m) => m,
        // The child returned nothing meaningful (no `return context` at end
        // of script, or no script at all). Hand the parent an empty context
        // — they can choose to ignore it.
        _ => ContextMap::new(),
    };

    // Build a fresh RenderContext seeded with the child's resulting map,
    // then construct a Context against it. This is the same path
    // Context.new() takes from a script.
    let child_render_context = RenderContext::new(
        render_context.destination().to_path_buf(),
        map,
    );
    Ok(Context::new(archetect.clone(), child_render_context))
}

/// Parse variadic args for catalog.render(): (context), (path, context), or (path, context, opts).
fn parse_catalog_render_args(args: &mlua::MultiValue) -> LuaResult<(Option<String>, AnyUserData, Option<Table>)> {
    match args.len() {
        1 => {
            // catalog.render(context)
            let context_ud = extract_userdata(args, 0, "catalog.render(context)")?;
            Ok((None, context_ud, None))
        }
        2 => {
            // Could be (path, context) or (context, opts)
            if let Some(Value::String(_)) = args.get(0) {
                let path = extract_string(args, 0, "catalog.render(path, context)")?;
                let context_ud = extract_userdata(args, 1, "catalog.render(path, context)")?;
                Ok((Some(path), context_ud, None))
            } else {
                let context_ud = extract_userdata(args, 0, "catalog.render(context, opts)")?;
                let opts = extract_table(args, 1, "catalog.render(context, opts)")?;
                Ok((None, context_ud, Some(opts)))
            }
        }
        3 => {
            // (path, context, opts)
            let path = extract_string(args, 0, "catalog.render(path, context, opts)")?;
            let context_ud = extract_userdata(args, 1, "catalog.render(path, context, opts)")?;
            let opts = extract_table(args, 2, "catalog.render(path, context, opts)")?;
            Ok((Some(path), context_ud, Some(opts)))
        }
        n => Err(LuaError::RuntimeError(format!(
            "catalog.render() takes 1-3 arguments, got {}", n
        ))),
    }
}

fn extract_string(args: &mlua::MultiValue, idx: usize, context: &str) -> LuaResult<String> {
    args.get(idx)
        .and_then(|v| match v {
            Value::String(s) => Some(s.to_string_lossy().to_string()),
            _ => None,
        })
        .ok_or_else(|| LuaError::RuntimeError(format!("{}: arg {} must be a string", context, idx + 1)))
}

fn extract_userdata(args: &mlua::MultiValue, idx: usize, context: &str) -> LuaResult<AnyUserData> {
    args.get(idx)
        .and_then(|v| match v {
            Value::UserData(ud) => Some(ud.clone()),
            _ => None,
        })
        .ok_or_else(|| LuaError::RuntimeError(format!("{}: arg {} must be a Context", context, idx + 1)))
}

fn extract_table(args: &mlua::MultiValue, idx: usize, context: &str) -> LuaResult<Table> {
    args.get(idx)
        .and_then(|v| match v {
            Value::Table(t) => Some(t.clone()),
            _ => None,
        })
        .ok_or_else(|| LuaError::RuntimeError(format!("{}: arg {} must be a table", context, idx + 1)))
}

// ── runtime module ──────────────────────────────────────────────────

// register_runtime_module removed — `is_offline`, `is_headless`, and
// `locals_enabled` are now fields on `archetect.*`. Keep one namespace
// per concept (binary state).

// ── env module ──────────────────────────────────────────────────────

/// Build the `archetect.env` sub-table. Reached as `archetect.env` from
/// Lua — there is no top-level `env` global. See register_archetect_module
/// for the attachment point.
fn build_env_table(lua: &Lua) -> LuaResult<Table> {
    let env_table = lua.create_table()?;
    env_table.set("os", std::env::consts::OS)?;
    env_table.set("arch", std::env::consts::ARCH)?;
    env_table.set("family", std::env::consts::FAMILY)?;
    env_table.set("is_unix", std::env::consts::FAMILY == "unix")?;
    env_table.set("is_windows", std::env::consts::FAMILY == "windows")?;
    env_table.set("is_macos", std::env::consts::OS == "macos")?;
    Ok(env_table)
}

/// Build the `archetype.switches` sub-table with a single `is_enabled(name)`
/// method. Reached as `archetype.switches.is_enabled(...)` from Lua — there
/// is no top-level `switches` global.
fn build_switches_table(lua: &Lua, render_context: &RenderContext) -> LuaResult<Table> {
    let switches_table = lua.create_table()?;
    let switches = render_context.switches().clone();
    switches_table.set(
        "is_enabled",
        lua.create_function(move |_, name: String| Ok(switches.contains(&name)))?,
    )?;
    Ok(switches_table)
}

// ── Lua-native directory module ─────────────────────────────────────

fn register_lua_directory_module(
    lua: &Lua,
    archetype: &Archetype,
    archetect: &Archetect,
    render_context: &RenderContext,
    filters: &Table,
    cache: Rc<RefCell<TemplateCache>>,
) -> LuaResult<()> {
    let directory_table = lua.create_table()?;

    let arch = archetype.clone();
    let arc = archetect.clone();
    let ctx = render_context.clone();
    let filters = filters.clone();
    let cache = cache.clone();

    directory_table.set(
        "render",
        lua.create_function(
            move |lua, (dir_name, context_ud, opts): (String, AnyUserData, Option<Table>)| {
                let context = context_ud.borrow::<Context>()?;
                let ctx_table = context.to_lua_table(lua)?;

                // Phase 1, commit 6: directory.render(path, ...) is
                // now resolved directly against the archetype root.
                // There is no longer a templating.content prefix.
                let source = arch.root().join(&dir_name);
                let mut destination = ctx.destination().to_owned();

                if let Some(ref opts) = opts {
                    if let Ok(dest_str) = opts.get::<String>("destination".to_string()) {
                        let dest_str = restrict_path(&dest_str)?;
                        destination = destination.join(dest_str);
                    }
                }

                let overwrite_policy = extract_overwrite_policy(&opts);
                let mut cache = cache.borrow_mut();

                lua_render::lua_render_directory(
                    lua,
                    &arc,
                    &ctx_table,
                    &filters,
                    source,
                    destination,
                    overwrite_policy,
                    &mut cache,
                )
                .map_err(|e| LuaError::RuntimeError(format!("Render error: {}", e)))
            },
        )?,
    )?;

    lua.globals().set("directory", directory_table)?;
    Ok(())
}

// ── file module ─────────────────────────────────────────────────────
//
// Single-file counterparts to `directory.render`. Exposes:
//
//   file.exists(path, opts?)   → boolean
//   file.read(path, opts?)     → string
//   file.render(src, ctx, opts?) → ()
//
// Path resolution:
//   - Default scope: archetype source root (consistent with
//     directory.render, archetype.render).
//   - opts.scope = "cwd": invocation working directory (for the rare
//     case where an archetype needs to read from the caller's project).
//
// Sandbox (applies to both scopes):
//   - No absolute paths.
//   - No `..` segments.
//   - No `~` home expansion.
//
// Rationale for the default: .archetect.yaml in CWD is already
// auto-detected into configuration, and `-A answers.yaml` handles
// explicit per-invocation context, so the cwd-scoped read is a rare
// escape hatch rather than the common case.

fn register_lua_file_module(
    lua: &Lua,
    archetype: &Archetype,
    archetect: &Archetect,
    render_context: &RenderContext,
    filters: &Table,
    cache: Rc<RefCell<TemplateCache>>,
) -> LuaResult<()> {
    let file_table = lua.create_table()?;

    let archetype_root = archetype.root().to_owned();
    let destination = render_context.destination().to_owned();

    // file.exists(path, opts?)
    {
        let archetype_root = archetype_root.clone();
        let destination = destination.clone();
        file_table.set(
            "exists",
            lua.create_function(move |_, (path, opts): (String, Option<Table>)| -> LuaResult<bool> {
                let resolved = resolve_file_path(&archetype_root, &destination, &path, &opts)?;
                Ok(resolved.exists())
            })?,
        )?;
    }

    // file.read(path, opts?)
    {
        let archetype_root = archetype_root.clone();
        let destination = destination.clone();
        file_table.set(
            "read",
            lua.create_function(move |_, (path, opts): (String, Option<Table>)| -> LuaResult<String> {
                let resolved = resolve_file_path(&archetype_root, &destination, &path, &opts)?;
                if !resolved.is_file() {
                    return Err(LuaError::RuntimeError(format!(
                        "file.read: not a regular file: {}",
                        path
                    )));
                }
                std::fs::read_to_string(resolved.as_std_path()).map_err(|err| {
                    LuaError::RuntimeError(format!("file.read: {}: {}", path, err))
                })
            })?,
        )?;
    }

    // file.render(source, context, opts?)
    //
    // Symmetric with template.render: returns the rendered string by
    // default; writes to disk when opts.destination is supplied.
    //
    //   local s = file.render("snippet.atl", ctx)
    //   file.render("README.md", ctx, { destination = "README.md" })
    //
    // Source always resolves against the archetype root — no `within`
    // here (rendering from the caller's cwd would be a footgun).
    {
        let archetype_root = archetype_root.clone();
        let arc = archetect.clone();
        let ctx_default_dest = render_context.destination().to_owned();
        let filters = filters.clone();
        let cache = cache.clone();
        file_table.set(
            "render",
            lua.create_function(
                move |lua, (source, context_ud, opts): (String, AnyUserData, Option<Table>)| -> LuaResult<Option<String>> {
                    restrict_path(&source)?;
                    if std::path::Path::new(&source).is_absolute() {
                        return Err(LuaError::RuntimeError(format!(
                            "file.render: absolute paths not allowed: {}",
                            source
                        )));
                    }
                    let source_path = archetype_root.join(&source);
                    if !source_path.is_file() {
                        return Err(LuaError::RuntimeError(format!(
                            "file.render: source is not a regular file: {}",
                            source
                        )));
                    }

                    let context = context_ud.borrow::<Context>()?;
                    let ctx_table = context.to_lua_table(lua)?;

                    // Extract optional destination. If absent → return the
                    // rendered string. If present → write to that path
                    // (sandbox-checked) and return nil.
                    let dest_opt: Option<String> = match opts.as_ref() {
                        Some(o) => match o.get::<String>("destination".to_string()) {
                            Ok(d) => {
                                restrict_path(&d)?;
                                if std::path::Path::new(&d).is_absolute() {
                                    return Err(LuaError::RuntimeError(format!(
                                        "file.render: destination must be relative: {}",
                                        d
                                    )));
                                }
                                Some(d)
                            }
                            Err(_) => None,
                        },
                        None => None,
                    };

                    let mut cache = cache.borrow_mut();

                    if let Some(dest_rel) = dest_opt {
                        let destination = ctx_default_dest.join(&dest_rel);
                        let overwrite_policy = extract_overwrite_policy(&opts);
                        crate::templating::atl::render::lua_render_file(
                            lua,
                            &arc,
                            &ctx_table,
                            &filters,
                            &source_path,
                            &destination,
                            overwrite_policy,
                            &mut cache,
                        )
                        .map_err(|e| LuaError::RuntimeError(format!("Render error: {}", e)))?;
                        Ok(None)
                    } else {
                        // No destination → render and return the string.
                        let rendered = crate::templating::atl::render::lua_render_contents(
                            lua,
                            &source_path,
                            &ctx_table,
                            &filters,
                            &mut cache,
                        )
                        .map_err(|e| LuaError::RuntimeError(format!("Render error: {}", e)))?;
                        Ok(Some(rendered))
                    }
                },
            )?,
        )?;
    }

    lua.globals().set("file", file_table)?;
    Ok(())
}

// ── Location enum ──────────────────────────────────────────────────
//
// Typed enum for file-resolution scope. Exposed in Lua as:
//   Location.Archetype     (default — archetype source root)
//   Location.Destination   (where files are being rendered — tracks `-d`)
//   Location.Cwd           (actual process working directory)
//
// Used via:  file.read("foo.yaml", { within = Location.Destination })
//
// Note on `Cwd` vs `Destination`: these coincide when the user invokes
// `archetect render` without `-d`, but diverge when `-d /some/other/path`
// is supplied. Scripts that want "the root of what we're rendering"
// should use `Location.Destination` — `Location.Cwd` is reserved for
// the rare case where an archetype genuinely needs to touch the caller's
// shell cwd regardless of where the render is going.

#[derive(Copy, Clone, Debug)]
enum FileLocation {
    Archetype,
    Destination,
    Cwd,
}

impl mlua::UserData for FileLocation {}

fn register_location_constants(lua: &Lua) -> LuaResult<()> {
    let table = lua.create_table()?;
    table.set("Archetype", FileLocation::Archetype)?;
    table.set("Destination", FileLocation::Destination)?;
    table.set("Cwd", FileLocation::Cwd)?;
    lua.globals().set("Location", table)?;
    Ok(())
}

/// Resolve a Lua-supplied relative path against one of the configured
/// `FileLocation` roots, applying `restrict_path` sandboxing plus
/// absolute-path rejection.
///
/// - `Location.Archetype` (default): path is relative to the archetype
///   source root. This is the same anchor `directory.render` uses.
/// - `Location.Destination`: path is relative to where files are being
///   rendered to (tracks the `-d` flag). The correct choice for scripts
///   that inspect the output tree — e.g. "does `<repo_name>/.git` exist?"
/// - `Location.Cwd`: path is relative to the actual process cwd,
///   regardless of `-d`. Escape hatch for rare cases where the script
///   genuinely needs the caller's shell position.
fn resolve_file_path(
    archetype_root: &camino::Utf8Path,
    destination: &camino::Utf8Path,
    path: &str,
    opts: &Option<Table>,
) -> LuaResult<camino::Utf8PathBuf> {
    restrict_path(path)?;
    if std::path::Path::new(path).is_absolute() {
        return Err(LuaError::RuntimeError(format!(
            "file.*: absolute paths not allowed: {}",
            path
        )));
    }

    let location = opts
        .as_ref()
        .and_then(|o| {
            o.get::<mlua::Value>("within".to_string()).ok().and_then(|v| {
                if let mlua::Value::UserData(ud) = v {
                    ud.borrow::<FileLocation>().ok().map(|l| *l)
                } else {
                    None
                }
            })
        })
        .unwrap_or(FileLocation::Archetype);

    let root: camino::Utf8PathBuf = match location {
        FileLocation::Archetype => archetype_root.to_owned(),
        FileLocation::Destination => destination.to_owned(),
        FileLocation::Cwd => {
            let cwd = std::env::current_dir().map_err(|err| {
                LuaError::RuntimeError(format!("file.*: could not read cwd: {}", err))
            })?;
            camino::Utf8PathBuf::from_path_buf(cwd).map_err(|bad| {
                LuaError::RuntimeError(format!("file.*: cwd is not valid UTF-8: {:?}", bad))
            })?
        }
    };

    Ok(root.join(path))
}

// ── Lua-native template module ──────────────────────────────────────

fn register_lua_template_module(
    lua: &Lua,
    archetect: &Archetect,
    render_context: &RenderContext,
    filters: &Table,
) -> LuaResult<()> {
    let template_table = lua.create_table()?;

    // Phase 8.3: capture the filter table directly via closures rather
    // than bridging through a `__atl_filters` Lua global. Both `render`
    // and `register_filters` close over independent clones of the same
    // underlying Lua table — Lua tables are reference types, so writes
    // through one clone are visible through the other.
    //
    // template.render(tmpl, ctx, opts?)
    //
    // Symmetric with file.render: returns the rendered string by
    // default; writes to disk when opts.destination is supplied.
    //
    //   local s = template.render("hello {{ name }}", ctx)
    //   template.render("hello {{ name }}", ctx, { destination = "out.txt" })
    let filters_for_render = filters.clone();
    let arc = archetect.clone();
    let ctx_default_dest = render_context.destination().to_owned();
    template_table.set(
        "render",
        lua.create_function(move |lua, (tmpl, context_ud, opts): (String, AnyUserData, Option<Table>)| -> LuaResult<Option<String>> {
            let context = context_ud.borrow::<Context>()?;
            let ctx_table = context.to_lua_table(lua)?;

            let compiled = crate::templating::atl::TemplateCompiler::compile(&tmpl, "<inline>")
                .map_err(|e| LuaError::RuntimeError(format!("Template compile error: {}", e)))?;

            let func: mlua::Function = lua.load(&compiled.source).eval()
                .map_err(|e| LuaError::RuntimeError(format!("Template load error: {}", e)))?;

            let rendered: String = func.call::<String>((ctx_table, filters_for_render.clone()))
                .map_err(|e| LuaError::RuntimeError(format!("Template error: {}", e)))?;

            // No opts.destination → return the rendered string.
            // opts.destination present → write to disk and return nil.
            let dest_opt: Option<String> = match opts.as_ref() {
                Some(o) => match o.get::<String>("destination".to_string()) {
                    Ok(d) => {
                        restrict_path(&d)?;
                        if std::path::Path::new(&d).is_absolute() {
                            return Err(LuaError::RuntimeError(format!(
                                "template.render: destination must be relative: {}",
                                d
                            )));
                        }
                        Some(d)
                    }
                    Err(_) => None,
                },
                None => None,
            };

            if let Some(dest_rel) = dest_opt {
                let destination = ctx_default_dest.join(&dest_rel);
                let overwrite_policy = extract_overwrite_policy(&opts);
                // Send the same WriteDirectory + WriteFile pair file.render
                // uses, going through the IO channel so all the existing
                // policy / hook handling applies.
                if let Some(parent) = destination.parent() {
                    if !parent.as_str().is_empty() {
                        arc.request(archetect_api::ScriptMessage::WriteDirectory(
                            archetect_api::WriteDirectoryInfo {
                                path: parent.to_string(),
                            },
                        )).map_err(|e| LuaError::RuntimeError(format!("Write error: {}", e)))?;
                        match arc.response().map_err(|e| LuaError::RuntimeError(format!("{}", e)))? {
                            archetect_api::ClientMessage::Ack => {}
                            archetect_api::ClientMessage::Error(msg) => {
                                return Err(LuaError::RuntimeError(format!(
                                    "template.render: failed to create destination dir {}: {}",
                                    parent, msg
                                )));
                            }
                            other => return Err(LuaError::RuntimeError(format!("Unexpected response: {:?}", other))),
                        }
                    }
                }
                arc.request(archetect_api::ScriptMessage::WriteFile(
                    archetect_api::WriteFileInfo {
                        destination: destination.to_string(),
                        contents: rendered.into_bytes(),
                        existing_file_policy: overwrite_policy.into(),
                    },
                )).map_err(|e| LuaError::RuntimeError(format!("Write error: {}", e)))?;
                match arc.response().map_err(|e| LuaError::RuntimeError(format!("{}", e)))? {
                    archetect_api::ClientMessage::Ack => Ok(None),
                    archetect_api::ClientMessage::Error(msg) => Err(LuaError::RuntimeError(format!(
                        "template.render: write failed for {}: {}", destination, msg
                    ))),
                    other => Err(LuaError::RuntimeError(format!("Unexpected response: {:?}", other))),
                }
            } else {
                Ok(Some(rendered))
            }
        })?,
    )?;

    let filters_for_register = filters.clone();
    template_table.set(
        "register_filters",
        lua.create_function(move |_, custom_filters: Table| {
            for pair in custom_filters.pairs::<String, mlua::Function>() {
                let (name, func) = pair?;
                filters_for_register.set(name, func)?;
            }
            Ok(())
        })?,
    )?;

    lua.globals().set("template", template_table)?;
    Ok(())
}

/// Coerce a Lua value into a string for a scalar filter (case conversions, etc.).
///
/// String/Integer/Number/Boolean coerce to their natural string forms. Nil
/// becomes the empty string. Non-scalar values (tables, functions, threads,
/// userdata) raise an explicit runtime error so authors get a clear message
/// instead of garbled `Debug`-formatted output leaking into a generated file.
fn coerce_scalar_for_filter(value: &Value, filter_name: &str) -> LuaResult<String> {
    match value {
        Value::String(s) => Ok(s.to_string_lossy().to_string()),
        Value::Integer(i) => Ok(i.to_string()),
        Value::Number(n) => Ok(n.to_string()),
        Value::Boolean(b) => Ok(b.to_string()),
        Value::Nil => Ok(String::new()),
        other => Err(LuaError::RuntimeError(format!(
            "filter `{}` expects a scalar value, got {}",
            filter_name,
            other.type_name()
        ))),
    }
}

/// Create the built-in filter table for the Lua template engine.
/// These are the inflection filters ported as Lua functions.
fn create_builtin_filters(lua: &Lua) -> LuaResult<Table> {
    let filters = lua.create_table()?;

    macro_rules! add_string_filter {
        ($name:expr, $func:expr) => {
            filters.set(
                $name,
                lua.create_function(|_, value: Value| {
                    let s = coerce_scalar_for_filter(&value, $name)?;
                    Ok($func(&s))
                })?,
            )?;
        };
    }

    add_string_filter!("camel_case", archetect_inflections::to_camel_case);
    add_string_filter!("class_case", archetect_inflections::to_class_case);
    add_string_filter!("cobol_case", archetect_inflections::to_cobol_case);
    add_string_filter!("constant_case", archetect_inflections::to_screaming_snake_case);
    add_string_filter!("directory_case", archetect_inflections::to_directory_case);
    add_string_filter!("kebab_case", archetect_inflections::to_kebab_case);
    add_string_filter!("lower_case", |s: &str| s.to_lowercase());
    add_string_filter!("upper_case", |s: &str| s.to_uppercase());
    add_string_filter!("lower", |s: &str| s.to_lowercase());
    add_string_filter!("upper", |s: &str| s.to_uppercase());
    add_string_filter!("pascal_case", archetect_inflections::to_pascal_case);
    add_string_filter!("package_case", archetect_inflections::to_package_case);
    add_string_filter!("sentence_case", archetect_inflections::to_sentence_case);
    add_string_filter!("snake_case", archetect_inflections::to_snake_case);
    add_string_filter!("train_case", archetect_inflections::to_train_case);
    add_string_filter!("title_case", archetect_inflections::to_title_case);
    add_string_filter!("pluralize", archetect_inflections::to_plural);
    add_string_filter!("plural", archetect_inflections::to_plural);
    add_string_filter!("singularize", archetect_inflections::to_singular);
    add_string_filter!("singular", archetect_inflections::to_singular);
    add_string_filter!("ordinalize", archetect_inflections::ordinalize);
    add_string_filter!("deordinalize", archetect_inflections::deordinalize);

    // Register the Phase 3 built-in primitives (strings, collections,
    // datetime, uuids, paths). Each entry registered here is reachable
    // both as `{{ x | foo }}` and `{{ foo(x) }}` per the filter/function
    // symmetry implemented in the template engine compiler.
    crate::templating::atl::builtins::register_all(lua, &filters)?;

    Ok(filters)
}

// ── format module ───────────────────────────────────────────────────

fn register_format_module(lua: &Lua) -> LuaResult<()> {
    let format_table = lua.create_table()?;

    // format.to_json(value) → string
    format_table.set(
        "to_json",
        lua.create_function(|_, value: Value| {
            let json_value = lua_value_to_json(&value)?;
            serde_json::to_string_pretty(&json_value)
                .map_err(|e| LuaError::RuntimeError(format!("JSON serialization error: {}", e)))
        })?,
    )?;

    // format.to_yaml(value) → string
    format_table.set(
        "to_yaml",
        lua.create_function(|_, value: Value| {
            let json_value = lua_value_to_json(&value)?;
            serde_yaml::to_string(&json_value)
                .map_err(|e| LuaError::RuntimeError(format!("YAML serialization error: {}", e)))
        })?,
    )?;

    // format.to_toml(value) → string
    format_table.set(
        "to_toml",
        lua.create_function(|_, value: Value| {
            let json_value = lua_value_to_json(&value)?;
            toml::to_string_pretty(&json_value)
                .map_err(|e| LuaError::RuntimeError(format!("TOML serialization error: {}", e)))
        })?,
    )?;

    // format.from_yaml(string) → Lua table
    format_table.set(
        "from_yaml",
        lua.create_function(|lua, yaml_str: String| {
            let json_value: serde_json::Value = serde_yaml::from_str(&yaml_str)
                .map_err(|e| LuaError::RuntimeError(format!("YAML parse error: {}", e)))?;
            json_to_lua_value(lua, &json_value)
        })?,
    )?;

    // format.from_json(string) → Lua table
    format_table.set(
        "from_json",
        lua.create_function(|lua, json_str: String| {
            let json_value: serde_json::Value = serde_json::from_str(&json_str)
                .map_err(|e| LuaError::RuntimeError(format!("JSON parse error: {}", e)))?;
            json_to_lua_value(lua, &json_value)
        })?,
    )?;

    // format.from_toml(string) → Lua table
    format_table.set(
        "from_toml",
        lua.create_function(|lua, toml_str: String| {
            let json_value: serde_json::Value = toml::from_str(&toml_str)
                .map_err(|e| LuaError::RuntimeError(format!("TOML parse error: {}", e)))?;
            json_to_lua_value(lua, &json_value)
        })?,
    )?;

    // Backwards compat aliases: format.yaml(), format.json(), format.toml()
    // These are the original serialization-only names. Kept to avoid breaking
    // existing archetypes, but new code should prefer format.to_yaml() etc.
    format_table.set("yaml", format_table.get::<mlua::Function>("to_yaml")?)?;
    format_table.set("json", format_table.get::<mlua::Function>("to_json")?)?;
    format_table.set("toml", format_table.get::<mlua::Function>("to_toml")?)?;

    lua.globals().set("format", format_table)?;
    Ok(())
}

/// Convert a serde_json::Value to a Lua Value.
fn json_to_lua_value(lua: &Lua, value: &serde_json::Value) -> LuaResult<Value> {
    match value {
        serde_json::Value::Null => Ok(Value::Nil),
        serde_json::Value::Bool(b) => Ok(Value::Boolean(*b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(Value::Integer(i))
            } else if let Some(f) = n.as_f64() {
                Ok(Value::Number(f))
            } else {
                Ok(Value::Nil)
            }
        }
        serde_json::Value::String(s) => Ok(Value::String(lua.create_string(s)?)),
        serde_json::Value::Array(arr) => {
            let table = lua.create_table()?;
            for (i, item) in arr.iter().enumerate() {
                table.set(i + 1, json_to_lua_value(lua, item)?)?;
            }
            Ok(Value::Table(table))
        }
        serde_json::Value::Object(map) => {
            let table = lua.create_table()?;
            for (key, val) in map {
                table.set(key.as_str(), json_to_lua_value(lua, val)?)?;
            }
            Ok(Value::Table(table))
        }
    }
}

/// Convert a Lua Value to a serde_json::Value for serialization.
/// Accepts Context userdata (serializes its data) or plain Lua tables.
fn lua_value_to_json(value: &Value) -> LuaResult<serde_json::Value> {
    match value {
        Value::Nil => Ok(serde_json::Value::Null),
        Value::Boolean(b) => Ok(serde_json::Value::Bool(*b)),
        Value::Integer(i) => Ok(serde_json::Value::Number((*i).into())),
        Value::Number(n) => {
            serde_json::Number::from_f64(*n)
                .map(serde_json::Value::Number)
                .ok_or_else(|| LuaError::RuntimeError("Cannot serialize NaN/Infinity".into()))
        }
        Value::String(s) => Ok(serde_json::Value::String(s.to_string_lossy().to_string())),
        Value::Table(table) => {
            // Check if it's an array (sequential integer keys starting at 1)
            let len = table.raw_len();
            if len > 0 {
                let mut arr = Vec::new();
                for i in 1..=len {
                    let v: Value = table.raw_get(i)?;
                    arr.push(lua_value_to_json(&v)?);
                }
                Ok(serde_json::Value::Array(arr))
            } else {
                let mut map = serde_json::Map::new();
                for pair in table.pairs::<Value, Value>() {
                    let (k, v) = pair?;
                    let key = match &k {
                        Value::String(s) => s.to_string_lossy().to_string(),
                        Value::Integer(i) => i.to_string(),
                        _ => continue,
                    };
                    map.insert(key, lua_value_to_json(&v)?);
                }
                Ok(serde_json::Value::Object(map))
            }
        }
        Value::UserData(ud) => {
            // Handle Context userdata
            if let Ok(ctx) = ud.borrow::<Context>() {
                let context_map = ctx.to_context_map();
                let mut map = serde_json::Map::new();
                for (k, v) in &context_map {
                    let json_val: serde_json::Value = v.clone().into();
                    map.insert(k.clone(), json_val);
                }
                return Ok(serde_json::Value::Object(map));
            }
            Ok(serde_json::Value::Null)
        }
        _ => Ok(serde_json::Value::Null),
    }
}

// ── exit function ───────────────────────────────────────────────────

/// Sentinel error message used to distinguish clean exit() from real errors.
pub(crate) const EXIT_SENTINEL: &str = "__archetect_exit__";

fn register_exit(lua: &Lua) -> LuaResult<()> {
    lua.globals().set(
        "exit",
        lua.create_function(|_, ()| -> LuaResult<()> {
            Err(LuaError::RuntimeError(EXIT_SENTINEL.to_string()))
        })?,
    )?;
    Ok(())
}

// ── log module ──────────────────────────────────────────────────────

fn register_log(lua: &Lua, archetect: &Archetect) -> LuaResult<()> {
    let log_table = lua.create_table()?;

    let arc = archetect.clone();
    log_table.set("info", lua.create_function(move |_, msg: String| {
        let _ = arc.request(archetect_api::ScriptMessage::LogInfo(msg));
        Ok(())
    })?)?;

    let arc = archetect.clone();
    log_table.set("debug", lua.create_function(move |_, msg: String| {
        let _ = arc.request(archetect_api::ScriptMessage::LogDebug(msg));
        Ok(())
    })?)?;

    let arc = archetect.clone();
    log_table.set("warn", lua.create_function(move |_, msg: String| {
        let _ = arc.request(archetect_api::ScriptMessage::LogWarn(msg));
        Ok(())
    })?)?;

    let arc = archetect.clone();
    log_table.set("error", lua.create_function(move |_, msg: String| {
        let _ = arc.request(archetect_api::ScriptMessage::LogError(msg));
        Ok(())
    })?)?;

    let arc = archetect.clone();
    log_table.set("trace", lua.create_function(move |_, msg: String| {
        let _ = arc.request(archetect_api::ScriptMessage::LogTrace(msg));
        Ok(())
    })?)?;

    lua.globals().set("log", log_table)?;
    Ok(())
}

// ── output module ───────────────────────────────────────────────────

fn register_output(lua: &Lua, archetect: &Archetect) -> LuaResult<()> {
    let output_table = lua.create_table()?;

    let arc = archetect.clone();
    output_table.set("print", lua.create_function(move |_, msg: String| {
        let _ = arc.request(archetect_api::ScriptMessage::Print(msg));
        Ok(())
    })?)?;

    let arc = archetect.clone();
    output_table.set("banner", lua.create_function(move |_, msg: String| {
        let _ = arc.request(archetect_api::ScriptMessage::Display(msg));
        Ok(())
    })?)?;

    lua.globals().set("output", output_table)?;
    Ok(())
}

// ── Existing enum constants ─────────────────────────────────────────

impl mlua::UserData for OverwritePolicy {}

fn register_existing_constants(lua: &Lua) -> LuaResult<()> {
    let table = lua.create_table()?;
    table.set("Overwrite", OverwritePolicy::Overwrite)?;
    table.set("Preserve", OverwritePolicy::Preserve)?;
    table.set("Prompt", OverwritePolicy::Prompt)?;
    table.set("Error", OverwritePolicy::Error)?;
    lua.globals().set("Existing", table)?;
    Ok(())
}

// ── Helpers ─────────────────────────────────────────────────────────

fn extract_overwrite_policy(opts: &Option<Table>) -> OverwritePolicy {
    if let Some(ref opts) = opts {
        if let Ok(mlua::Value::UserData(ud)) = opts.get::<mlua::Value>("if_exists".to_string()) {
            if let Ok(policy) = ud.borrow::<OverwritePolicy>() {
                return *policy;
            }
        }
    }
    OverwritePolicy::Preserve
}

/// Reject paths that attempt directory traversal or home-relative access.
fn restrict_path(path: &str) -> LuaResult<&str> {
    if path.starts_with("~/") || path.starts_with("../") || path.contains("/../") || path.ends_with("/..") {
        return Err(LuaError::RuntimeError(format!(
            "Path manipulation not allowed: '{}'", path
        )));
    }
    Ok(path)
}

/// Convert a ContextMap to a Lua table for the answers() function.
fn context_map_to_lua_table(lua: &Lua, map: &archetect_api::ContextMap) -> LuaResult<Value> {
    let table = lua.create_table()?;
    for (key, value) in map {
        let lua_value = context_value_to_lua(lua, value)?;
        table.set(key.as_str(), lua_value)?;
    }
    Ok(Value::Table(table))
}

/// Convert a ContextValue to a Lua value.
fn context_value_to_lua(lua: &Lua, value: &archetect_api::ContextValue) -> LuaResult<Value> {
    match value {
        archetect_api::ContextValue::String(s) => Ok(Value::String(lua.create_string(s)?)),
        archetect_api::ContextValue::Integer(i) => Ok(Value::Integer(*i)),
        archetect_api::ContextValue::Float(f) => Ok(Value::Number(*f)),
        archetect_api::ContextValue::Boolean(b) => Ok(Value::Boolean(*b)),
        archetect_api::ContextValue::Array(arr) => {
            let table = lua.create_table()?;
            for (i, item) in arr.iter().enumerate() {
                table.set(i + 1, context_value_to_lua(lua, item)?)?;
            }
            Ok(Value::Table(table))
        }
        archetect_api::ContextValue::Map(map) => context_map_to_lua_table(lua, map),
        archetect_api::ContextValue::Nil => Ok(Value::Nil),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::templating::atl::TemplateCompiler;

    #[test]
    fn test_coerce_scalar_for_filter_string() {
        let lua = Lua::new();
        let value = Value::String(lua.create_string("hello").unwrap());
        assert_eq!(coerce_scalar_for_filter(&value, "upper_case").unwrap(), "hello");
    }

    #[test]
    fn test_coerce_scalar_for_filter_integer() {
        let value = Value::Integer(42);
        assert_eq!(coerce_scalar_for_filter(&value, "upper_case").unwrap(), "42");
    }

    #[test]
    fn test_coerce_scalar_for_filter_number() {
        let value = Value::Number(2.5);
        assert_eq!(coerce_scalar_for_filter(&value, "upper_case").unwrap(), "2.5");
    }

    #[test]
    fn test_coerce_scalar_for_filter_boolean() {
        let value = Value::Boolean(true);
        assert_eq!(coerce_scalar_for_filter(&value, "upper_case").unwrap(), "true");
    }

    #[test]
    fn test_coerce_scalar_for_filter_nil() {
        let value = Value::Nil;
        assert_eq!(coerce_scalar_for_filter(&value, "upper_case").unwrap(), "");
    }

    #[test]
    fn resolve_file_path_archetype_root() {
        let archetype = camino::Utf8PathBuf::from("/a/arc");
        let destination = camino::Utf8PathBuf::from("/a/dest");
        let got = resolve_file_path(&archetype, &destination, "foo/bar.txt", &None).unwrap();
        assert_eq!(got, camino::Utf8PathBuf::from("/a/arc/foo/bar.txt"));
    }

    #[test]
    fn resolve_file_path_destination() {
        // Location.Destination should resolve against the render destination,
        // diverging from cwd when `-d` points elsewhere. This is the core
        // fix — scripts that inspect the output tree need this anchor.
        let archetype = camino::Utf8PathBuf::from("/a/arc");
        let destination = camino::Utf8PathBuf::from("/a/dest");
        let lua = Lua::new();
        register_location_constants(&lua).unwrap();
        let location: mlua::Value = lua
            .load(r#"return Location.Destination"#)
            .eval()
            .unwrap();
        let opts = lua.create_table().unwrap();
        opts.set("within", location).unwrap();
        let got =
            resolve_file_path(&archetype, &destination, ".git", &Some(opts)).unwrap();
        assert_eq!(got, camino::Utf8PathBuf::from("/a/dest/.git"));
    }

    #[test]
    fn resolve_file_path_rejects_absolute() {
        let archetype = camino::Utf8PathBuf::from("/a/arc");
        let destination = camino::Utf8PathBuf::from("/a/dest");
        let err = resolve_file_path(&archetype, &destination, "/etc/passwd", &None)
            .unwrap_err();
        assert!(err.to_string().contains("absolute paths not allowed"));
    }

    #[test]
    fn test_coerce_scalar_for_filter_table_errors() {
        let lua = Lua::new();
        let value = Value::Table(lua.create_table().unwrap());
        let err = coerce_scalar_for_filter(&value, "upper_case").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("upper_case"), "error should name the filter: {}", msg);
        assert!(msg.contains("scalar"), "error should explain the constraint: {}", msg);
        assert!(msg.contains("table"), "error should name the offending type: {}", msg);
    }

    #[test]
    fn test_filter_coerces_integer_to_string() {
        // {{ count | upper_case }} with count=5 should produce "5", not "Integer(5)".
        let compiled = TemplateCompiler::compile("{{ count | upper_case }}", "test").unwrap();
        let lua = Lua::new();
        let func: mlua::Function = lua.load(&compiled.source).eval().unwrap();

        let ctx = lua.create_table().unwrap();
        ctx.set("count", 5).unwrap();
        let filters = create_builtin_filters(&lua).unwrap();

        let result: String = func.call::<String>((ctx, filters)).unwrap();
        assert_eq!(result, "5");
    }

    #[test]
    fn test_filter_coerces_boolean_to_string() {
        let compiled = TemplateCompiler::compile("{{ flag | upper_case }}", "test").unwrap();
        let lua = Lua::new();
        let func: mlua::Function = lua.load(&compiled.source).eval().unwrap();

        let ctx = lua.create_table().unwrap();
        ctx.set("flag", true).unwrap();
        let filters = create_builtin_filters(&lua).unwrap();

        // upper_case("true") → "TRUE"
        let result: String = func.call::<String>((ctx, filters)).unwrap();
        assert_eq!(result, "TRUE");
    }

    #[test]
    fn test_filter_rejects_table_input() {
        // {{ items | upper_case }} where items is a table should fail with a clear error.
        let compiled = TemplateCompiler::compile("{{ items | upper_case }}", "test").unwrap();
        let lua = Lua::new();
        let func: mlua::Function = lua.load(&compiled.source).eval().unwrap();

        let ctx = lua.create_table().unwrap();
        ctx.set("items", lua.create_table().unwrap()).unwrap();
        let filters = create_builtin_filters(&lua).unwrap();

        let result = func.call::<String>((ctx, filters));
        assert!(result.is_err(), "expected error, got {:?}", result);
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("upper_case"), "error should name the filter: {}", msg);
    }

    // ---------- Phase 3: built-in primitives ----------
    //
    // The helpers below render an ATL template through the full pipeline
    // (compile → eval → call) using the actual built-in filter table. Each
    // builtin is tested twice where applicable: once via the pipe form
    // (`{{ x | foo }}`) and once via the function form (`{{ foo(x) }}`),
    // confirming the filter/function symmetry from Phase 3.0.

    fn render_with(template: &str, setup: impl FnOnce(&Lua, &Table)) -> String {
        let compiled = TemplateCompiler::compile(template, "test").unwrap();
        let lua = Lua::new();
        let func: mlua::Function = lua.load(&compiled.source).eval().unwrap();
        let ctx = lua.create_table().unwrap();
        setup(&lua, &ctx);
        let filters = create_builtin_filters(&lua).unwrap();
        func.call::<String>((ctx, filters)).unwrap()
    }

    fn render_no_ctx(template: &str) -> String {
        render_with(template, |_, _| {})
    }

    // ---------- strings: default ----------

    #[test]
    fn test_default_returns_value_when_present() {
        let out = render_with("{{ name | default(\"anon\") }}", |_, ctx| {
            ctx.set("name", "Jimmie").unwrap();
        });
        assert_eq!(out, "Jimmie");
    }

    #[test]
    fn test_default_returns_fallback_for_nil() {
        let out = render_no_ctx("{{ name | default(\"anon\") }}");
        assert_eq!(out, "anon");
    }

    #[test]
    fn test_default_returns_fallback_for_empty_string() {
        let out = render_with("{{ name | default(\"anon\") }}", |_, ctx| {
            ctx.set("name", "").unwrap();
        });
        assert_eq!(out, "anon");
    }

    // ---------- strings: truncate ----------

    #[test]
    fn test_truncate_pipe_form() {
        let out = render_with("{{ name | truncate(5) }}", |_, ctx| {
            ctx.set("name", "abcdefghij").unwrap();
        });
        assert_eq!(out, "abcde…");
    }

    #[test]
    fn test_truncate_function_form() {
        // Filter/function symmetry: same builtin via call syntax.
        let out = render_with("{{ truncate(name, 5) }}", |_, ctx| {
            ctx.set("name", "abcdefghij").unwrap();
        });
        assert_eq!(out, "abcde…");
    }

    #[test]
    fn test_truncate_with_custom_suffix() {
        let out = render_with(r#"{{ name | truncate(5, "...") }}"#, |_, ctx| {
            ctx.set("name", "abcdefghij").unwrap();
        });
        assert_eq!(out, "abcde...");
    }

    #[test]
    fn test_truncate_short_string_unchanged() {
        let out = render_with("{{ name | truncate(20) }}", |_, ctx| {
            ctx.set("name", "short").unwrap();
        });
        assert_eq!(out, "short");
    }

    // ---------- strings: replace, trim ----------

    #[test]
    fn test_replace() {
        let out = render_with(r#"{{ name | replace("a", "b") }}"#, |_, ctx| {
            ctx.set("name", "banana").unwrap();
        });
        assert_eq!(out, "bbnbnb");
    }

    #[test]
    fn test_trim() {
        let out = render_with("{{ name | trim }}", |_, ctx| {
            ctx.set("name", "  hello  ").unwrap();
        });
        assert_eq!(out, "hello");
    }

    #[test]
    fn test_trim_start_end() {
        let out = render_with("{{ name | trim_start }}", |_, ctx| {
            ctx.set("name", "  hello  ").unwrap();
        });
        assert_eq!(out, "hello  ");

        let out = render_with("{{ name | trim_end }}", |_, ctx| {
            ctx.set("name", "  hello  ").unwrap();
        });
        assert_eq!(out, "  hello");
    }

    // ---------- strings: indent, string_repeat ----------

    #[test]
    fn test_indent() {
        let out = render_with("{{ block | indent(4) }}", |_, ctx| {
            ctx.set("block", "line1\nline2\nline3").unwrap();
        });
        assert_eq!(out, "    line1\n    line2\n    line3");
    }

    #[test]
    fn test_string_repeat_pipe_and_function() {
        let pipe = render_no_ctx(r#"{{ "ab" | string_repeat(3) }}"#);
        let call = render_no_ctx(r#"{{ string_repeat("ab", 3) }}"#);
        assert_eq!(pipe, "ababab");
        assert_eq!(call, "ababab");
    }

    // ---------- strings: split, length, concat ----------

    #[test]
    fn test_split_then_length() {
        // {{ csv | split(",") | length }}
        let out = render_with(r#"{{ csv | split(",") | length }}"#, |_, ctx| {
            ctx.set("csv", "a,b,c,d").unwrap();
        });
        assert_eq!(out, "4");
    }

    #[test]
    fn test_length_of_string() {
        let out = render_with("{{ name | length }}", |_, ctx| {
            ctx.set("name", "hello").unwrap();
        });
        assert_eq!(out, "5");
    }

    #[test]
    fn test_concat_function_form() {
        let out = render_with(r#"{{ concat(prefix, "-", name) }}"#, |_, ctx| {
            ctx.set("prefix", "p6m").unwrap();
            ctx.set("name", "service").unwrap();
        });
        assert_eq!(out, "p6m-service");
    }

    // ---------- collections: join, first, last ----------

    #[test]
    fn test_join() {
        let out = render_with(r#"{{ items | join(", ") }}"#, |lua, ctx| {
            let arr = lua.create_table().unwrap();
            arr.set(1, "a").unwrap();
            arr.set(2, "b").unwrap();
            arr.set(3, "c").unwrap();
            ctx.set("items", arr).unwrap();
        });
        assert_eq!(out, "a, b, c");
    }

    #[test]
    fn test_first_last() {
        let setup = |lua: &Lua, ctx: &Table| {
            let arr = lua.create_table().unwrap();
            arr.set(1, "alpha").unwrap();
            arr.set(2, "beta").unwrap();
            arr.set(3, "gamma").unwrap();
            ctx.set("items", arr).unwrap();
        };
        assert_eq!(render_with("{{ items | first }}", setup), "alpha");
        assert_eq!(render_with("{{ items | last }}", setup), "gamma");
    }

    // ---------- collections: sort, reverse, unique ----------

    #[test]
    fn test_sort() {
        let out = render_with(r#"{{ items | sort | join(",") }}"#, |lua, ctx| {
            let arr = lua.create_table().unwrap();
            arr.set(1, "charlie").unwrap();
            arr.set(2, "alpha").unwrap();
            arr.set(3, "bravo").unwrap();
            ctx.set("items", arr).unwrap();
        });
        assert_eq!(out, "alpha,bravo,charlie");
    }

    #[test]
    fn test_reverse() {
        let out = render_with(r#"{{ items | reverse | join(",") }}"#, |lua, ctx| {
            let arr = lua.create_table().unwrap();
            arr.set(1, "a").unwrap();
            arr.set(2, "b").unwrap();
            arr.set(3, "c").unwrap();
            ctx.set("items", arr).unwrap();
        });
        assert_eq!(out, "c,b,a");
    }

    // ---------- collections: contains ----------

    #[test]
    fn test_contains_array_hit() {
        let out = render_with(r#"{{ contains(items, "TOC") }}"#, |lua, ctx| {
            let arr = lua.create_table().unwrap();
            arr.set(1, "TOC").unwrap();
            arr.set(2, "Admonish").unwrap();
            ctx.set("items", arr).unwrap();
        });
        assert_eq!(out, "true");
    }

    #[test]
    fn test_contains_array_miss() {
        let out = render_with(r#"{{ contains(items, "MermaidJS") }}"#, |lua, ctx| {
            let arr = lua.create_table().unwrap();
            arr.set(1, "TOC").unwrap();
            arr.set(2, "Admonish").unwrap();
            ctx.set("items", arr).unwrap();
        });
        assert_eq!(out, "false");
    }

    #[test]
    fn test_contains_string_substring() {
        let out = render_with(r#"{{ contains(name, "world") }}"#, |_, ctx| {
            ctx.set("name", "hello world").unwrap();
        });
        assert_eq!(out, "true");
    }

    #[test]
    fn test_contains_nil_haystack() {
        // Defensive: if the context var is undefined/nil, contains() returns
        // false instead of erroring. Useful for `{% if contains(features, "X") %}`
        // when `features` may not be set.
        let out = render_no_ctx(r#"{{ contains(features, "X") }}"#);
        assert_eq!(out, "false");
    }

    #[test]
    fn test_contains_in_template_logic_block() {
        // The Jinja-flavored use case: `{% if contains(items, "X") then %}`
        let template = r#"{% if contains(items, "TOC") then %}has TOC{% end %}"#;
        let out = render_with(template, |lua, ctx| {
            let arr = lua.create_table().unwrap();
            arr.set(1, "TOC").unwrap();
            ctx.set("items", arr).unwrap();
        });
        assert_eq!(out, "has TOC");
    }

    #[test]
    fn test_unique() {
        let out = render_with(r#"{{ items | unique | join(",") }}"#, |lua, ctx| {
            let arr = lua.create_table().unwrap();
            arr.set(1, "a").unwrap();
            arr.set(2, "b").unwrap();
            arr.set(3, "a").unwrap();
            arr.set(4, "c").unwrap();
            arr.set(5, "b").unwrap();
            ctx.set("items", arr).unwrap();
        });
        assert_eq!(out, "a,b,c");
    }

    // ---------- datetime ----------

    #[test]
    fn test_year_returns_current_year() {
        let out = render_no_ctx("{{ year() }}");
        let year: i32 = out.parse().expect("year should parse as integer");
        assert!(year >= 2026, "year should be at least 2026, got {}", year);
        assert!(year < 2100, "year should be reasonable, got {}", year);
    }

    #[test]
    fn test_today_format() {
        let out = render_no_ctx("{{ today() }}");
        // YYYY-MM-DD
        assert_eq!(out.len(), 10, "today() should be YYYY-MM-DD, got {}", out);
        assert_eq!(&out[4..5], "-");
        assert_eq!(&out[7..8], "-");
    }

    #[test]
    fn test_now_is_rfc3339() {
        let out = render_no_ctx("{{ now() }}");
        chrono::DateTime::parse_from_rfc3339(&out)
            .unwrap_or_else(|e| panic!("now() should be RFC3339, got {} ({})", out, e));
    }

    #[test]
    fn test_date_filter_extracts_year() {
        let out = render_no_ctx(r#"{{ today() | date("%Y") }}"#);
        let year: i32 = out.parse().expect("year format should parse");
        assert!(year >= 2026, "got year {}", year);
    }

    // ---------- uuids ----------

    #[test]
    fn test_uuid_v4_format() {
        let out = render_no_ctx("{{ uuid_v4() }}");
        // 8-4-4-4-12
        let parts: Vec<&str> = out.split('-').collect();
        assert_eq!(parts.len(), 5, "uuid should have 5 hyphenated parts: {}", out);
        assert_eq!(parts[0].len(), 8);
        assert_eq!(parts[1].len(), 4);
        assert_eq!(parts[2].len(), 4);
        assert_eq!(parts[3].len(), 4);
        assert_eq!(parts[4].len(), 12);
    }

    #[test]
    fn test_uuid_v7_format() {
        let out = render_no_ctx("{{ uuid_v7() }}");
        let parts: Vec<&str> = out.split('-').collect();
        assert_eq!(parts.len(), 5, "uuid should have 5 hyphenated parts: {}", out);
    }

    #[test]
    fn test_uuid_default_is_v4_shape() {
        let out = render_no_ctx("{{ uuid() }}");
        let parts: Vec<&str> = out.split('-').collect();
        assert_eq!(parts.len(), 5);
    }

    #[test]
    fn test_uuid_nil() {
        let out = render_no_ctx("{{ uuid_nil() }}");
        assert_eq!(out, "00000000-0000-0000-0000-000000000000");
    }

    // ---------- paths ----------

    #[test]
    fn test_path_join() {
        let out = render_no_ctx(r#"{{ path_join("a", "b", "c") }}"#);
        assert_eq!(out, "a/b/c");
    }

    #[test]
    fn test_path_join_collapses_separators() {
        let out = render_no_ctx(r#"{{ path_join("a/", "/b", "c/") }}"#);
        assert_eq!(out, "a/b/c");
    }

    #[test]
    fn test_basename() {
        let out = render_with("{{ p | basename }}", |_, ctx| {
            ctx.set("p", "/foo/bar/baz.rs").unwrap();
        });
        assert_eq!(out, "baz.rs");
    }

    #[test]
    fn test_dirname() {
        let out = render_with("{{ p | dirname }}", |_, ctx| {
            ctx.set("p", "/foo/bar/baz.rs").unwrap();
        });
        assert_eq!(out, "/foo/bar");
    }

    #[test]
    fn test_extname() {
        let out = render_with("{{ p | extname }}", |_, ctx| {
            ctx.set("p", "/foo/bar/baz.tar.gz").unwrap();
        });
        assert_eq!(out, ".gz");
    }

    #[test]
    fn test_extname_no_extension() {
        let out = render_with("{{ p | extname }}", |_, ctx| {
            ctx.set("p", "Makefile").unwrap();
        });
        assert_eq!(out, "");
    }

    #[test]
    fn test_extname_dotfile() {
        // Leading dot does not count as an extension.
        let out = render_with("{{ p | extname }}", |_, ctx| {
            ctx.set("p", ".gitignore").unwrap();
        });
        assert_eq!(out, "");
    }

    #[test]
    fn test_path_normalize_dots() {
        let out = render_with("{{ p | path_normalize }}", |_, ctx| {
            ctx.set("p", "a/./b/../c").unwrap();
        });
        assert_eq!(out, "a/c");
    }

    #[test]
    fn test_path_normalize_absolute() {
        let out = render_with("{{ p | path_normalize }}", |_, ctx| {
            ctx.set("p", "/a/b/../c/").unwrap();
        });
        assert_eq!(out, "/a/c");
    }
}
