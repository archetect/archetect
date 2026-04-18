# Archetect 3: Warts and Improvements

## Status snapshot (2026-04-17)

This is a diagnostic audit, not a phase plan. High-level state of the
major wart categories:

| Category | Status |
|---|---|
| Crash-prone error handling (`.unwrap()` panics) | in-progress (structured errors in place; some `.unwrap()` sites remain in less-critical paths) |
| Rhai engine error/context loss | shipped (Rhai removed entirely; Lua replaces it) |
| Missing manifest validation | shipped |
| Dry-run / preview mode | shipped (`--dry-run` / `-n` flag; intercepts file writes, git, shell, github.create_repo) |
| Archetype testing framework | planned |
| REPL / debugger | planned |
| Partial-render recovery / atomicity | planned |
| Component version pinning (git refs) | shipped for explicit refs; semver ranges remain planned |
| Conflict diff view on overwrite | shipped (terminal IO driver — unified diff before Prompt confirm and on Overwrite; binary files reported as size delta) |
| Archetype listing / search | shipped (`archetect ls` browses tree, `archetect search` does AND-keyword search; both share the MCP catalog index backend) |
| Documentation audit & rewrite | in-progress (see `documentation-audit-and-rewrite.md`) |

Items below are organized by severity. When acting on any of them, cross-
reference the table above before assuming they're still open.

## Context

A thorough audit of the Archetect v2 codebase, ~80 production archetypes (p6m-archetypes), and the documentation site revealed systemic issues across error handling, authoring experience, and missing features. This document catalogs them and prioritizes fixes for v3.

---

## Critical: Crash-Prone Error Handling

### 25+ panic paths via `.unwrap()` / `.expect()` in production code

**Answer file parsing** (`archetect-bin/src/answers/answers.rs`):
- `serde_yaml::from_str(&contents).unwrap()` on user-supplied YAML
- `serde_json::from_str(&contents).unwrap()` on user-supplied JSON
- `Engine::new().eval(&contents).unwrap()` on user-supplied Rhai
- `pairs.next().unwrap()` and `iter.next().unwrap()` on CLI key=value parsing
- Impact: Malformed input crashes the app instead of showing an error message

**IO channel** (`archetect-core/src/archetect/archetect.rs`):
- `.expect("Lock Error")` on mutex lock for response channel
- `.expect("Receive Error")` on channel receive
- Impact: Any channel issue crashes the app. No graceful error propagation.

**Source resolution** (`archetect-core/src/source.rs`):
- `.unwrap()` on `url.host_str()` — crashes if URL has no host
- `.unwrap()` on chrono timestamp parsing — crashes on invalid cached timestamps
- `.unwrap()` on mutex locks (lines 317, 348) — no error context
- `.unwrap()` on optional gitref without checking first (line 333)

**System layout** (`archetect-core/src/system.rs`):
- `UserDirs::new().unwrap()` — crashes if home directory doesn't exist
- Multiple `.unwrap()` on path-to-string conversions (lines 50, 55, 59, 110, 111)

**Configuration** (`archetect-core/src/configuration/configuration.rs`):
- `.expect("Unexpected error converting Configuration to yaml")` (line 90)
- `.unwrap()` on git config parsing (lines 134, 138, 144)
- `TimeDelta::try_seconds(...).expect(...)` on hardcoded config values

**Archetype rendering** (`archetect-core/src/archetype/archetype.rs`):
- `Utf8PathBuf::from_path_buf(entry.path()).unwrap()` — crashes on non-UTF8 paths (line 128)

### Rhai script errors lose all context

```rust
// archetect-core/src/archetype/archetype.rs lines 71-80
Err(error) => {
    return if let EvalAltResult::ErrorTerminated(_0, _1) = *error {
        Err(ArchetypeScriptError::ScriptAbortError)
    } else {
        self.archetect.request(CommandRequest::LogError(format!("{}", error)));
        Err(ArchetypeScriptError::ScriptAbortError)  // Line number, file, expression — all gone
    };
}
```

Users see "Archetype Script Aborted" with no line number, no file path, no indication of what failed. The actual Rhai error (which includes position information) is logged but then discarded.

### v3 fix

- Replace every `.unwrap()` / `.expect()` on user input paths with `?` or contextual error messages
- Propagate Rhai error position, file, and expression through to the user
- Define structured error types: `IoError`, `ScriptError { file, line, column, message, expression }`, `ManifestError`, `SourceError`
- Use `miette` or `ariadne` for rich terminal error rendering with source snippets

---

## Critical: Missing Manifest Validation

**`archetect-core/src/archetype/archetype_manifest.rs`** deserializes `archetype.yaml` via serde but performs zero validation:

| Not validated | Consequence |
|---------------|-------------|
| Script file exists | Discovered at render time, not load time |
| `modules_directory` path is valid | Silent failure when scripts try to import |
| `content_directory` path is valid | Render fails with generic IO error |
| `templates_directory` path is valid | Same |
| Component URLs/paths are valid | Fails only when component is actually rendered |
| `description` is non-empty | Silently accepted |
| `requires.archetect` version is compatible | Checked, but late — after fetch/clone |

**Wrong error types:**
- `ArchetypeManifestNotFound` is used when the *script file* is missing (not the manifest)
- No distinction between "manifest not found" and "manifest found but invalid"

### v3 fix

- Validate manifest immediately on load (`ArchetypeDirectory::new()`)
- Check all referenced paths exist
- Validate component URLs are syntactically valid
- Check requirements before any rendering work begins
- Provide clear, specific error messages for each validation failure

---

## High: Missing Features

### No dry-run / preview mode

Zero matches for "dry", "preview", or "--dry" in the codebase. Authors cannot see what will be generated without writing files to disk. They render into temporary directories and manually inspect output.

**v3 fix:** `archetect render --dry-run` that renders to an in-memory filesystem and displays a file tree (with optional content preview). Could also integrate with the IO protocol overhaul — WriteFile messages captured and displayed instead of written.

### No archetype testing framework

Authors have no way to write automated tests for their archetypes. The only "testing" is rendering with answer files and manually inspecting output. Some archetypes have `test_answers_complete.yaml` files, but there's no tooling to run them or validate results.

**v3 fix:** `archetect test` command that:
1. Renders archetype using answer file(s)
2. Compares output against snapshot directory
3. Diffs and reports mismatches
4. Can run in CI/CD
5. Supports multiple test cases per archetype (different answer combinations)

### No REPL / debugger

Debugging is `display(as_yaml(context))` behind `--switch debug-answers`. No breakpoints, no stepping, no variable inspection, no interactive evaluation.

**v3 fix:** `archetect repl <archetype>` that drops into an interactive scripting session with the archetype's context loaded. For Lua, this is trivial — Lua has a built-in REPL. LuaLS + editor integration provides further debugging.

### No partial render recovery

If rendering fails midway (template error, IO error, script abort), files are left in a partial state. No rollback, no resume. Zero matches for "rollback", "transaction", or "backup" in the codebase.

**v3 fix:** Render to a temporary directory, then atomically move to destination on success. On failure, temp directory is preserved for inspection but destination is untouched. The IO protocol overhaul enables this naturally — WriteFile messages are collected, then committed as a batch.

### No component version pinning

Archetype manifests reference components without version constraints:
```yaml
components:
  org-prompts: git@github.com:p6m-archetypes/org-prompts.archetype.git  # Always latest
```

If a dependency archetype changes its output format or API, there's no version constraint to prevent silent breakage.

**v3 fix:** Support version refs in component declarations:
```yaml
components:
  org-prompts:
    source: git@github.com:p6m-archetypes/org-prompts.archetype.git
    ref: v2.1.0  # or tag, branch, semver constraint
```

### No conflict diff view

Overwrite prompts are binary (yes/no). Users can't see what would change. In headless mode, conflicts are silently preserved with only a `trace!()` log — not visible at default log levels.

```rust
// archetect-core/src/archetype/archetype.rs
OverwritePolicy::Prompt => {
    if archetect.is_headless() {
        trace!("Preserving {:?}", destination);  // Silent!
    } else {
        // Interactive yes/no prompt — no diff shown
    }
}
```

**v3 fix:** Show a diff before the overwrite prompt. In headless mode, log skipped files at `INFO` level. Optionally support `--force` to overwrite without prompting.

### No archetype listing / search

Can't enumerate archetypes in a catalog without entering the interactive menu. No `archetect list` or programmatic search.

**v3 fix:** `archetect list <catalog-source>` that outputs catalog contents as a tree or flat list. Support `--json` for programmatic consumption. For the CodegenExtension, this becomes the database-backed search API.

---

## High: Documentation Gaps

| Gap | Severity | Current State |
|-----|----------|---------------|
| Templating engine reference | Critical | Placeholder: "Content will be added soon" |
| Complete scripting API reference | High | `prompt()` and `set()` documented. `render()`, `exec()`, git, github, archive functions missing. |
| CHANGELOG | High | No CHANGELOG.md across 18 releases. No documented breaking changes. |
| Migration guide | High | Nothing. No upgrade path documentation. |
| Authoring cookbook | Medium | No best practices, no complex examples, no troubleshooting guide. |

**v3 fix:** For v3, documentation should be a first-class deliverable:
- LuaLS annotation files serve as living API documentation
- `archetect docs` command that opens the documentation site
- Shipped examples directory with annotated archetypes
- CHANGELOG.md maintained from v3.0.0 onward
- Migration guide: v2 Rhai → v3 Lua

---

## Medium: UX Rough Edges

### Cache staleness is invisible

- Cache TTL stored in git config, not visible to users
- "Using cache" logged at `TRACE` level only — never seen at default verbosity
- No warning when serving stale content
- No command to check cache freshness

**v3 fix:** `archetect cache status` showing age of each cached source. Warn at `INFO` level when cache is older than TTL. Show cache age in render output.

### Config merging is opaque

5+ config sources merge silently:
1. Built-in defaults
2. `~/.archetect/archetect.yaml`
3. `~/.archetect/etc.d/*.yaml` (sorted — fragile ordering)
4. `./.archetect.yaml` in current dir
5. `./archetect.yaml` in current dir (both checked!)
6. `--config-file` CLI option
7. CLI flags

No indication which file overrides what. If both `.archetect.yaml` and `archetect.yaml` exist in the same directory, precedence is unclear. No validation that the merged result is sensible.

**v3 fix:** `archetect config show --annotated` showing each value and its source file. Reduce config file locations (pick one convention). Validate merged config.

### Error messages are generic

- `PathRenderError2` (naming cruft — why "2"?)
- `ArchetypeManifestNotFound` used for missing *script* files
- `SourceError::RemoteSourceError` when branch/tag doesn't exist — should say "branch not found"
- Answer file `InvalidFileType` with no hint about supported formats (.yaml, .json, .rhai)

**v3 fix:** Every error message should include: what happened, where (file/line), and what to do about it.

### Silent property access in Rhai

`context.typo_field` returns `()` instead of erroring. Observed in production scripts: `context.pnpm_intall` (typo for "install") silently evaluates to unit. Logic fails without any indication.

**v3 fix:** Lua has the same issue by default, but metatables can enforce strict access. The v3 Context object should error on access to undefined keys (opt-in or default).

---

## Medium: Template Engine Limitations

### Binary file detection is heuristic

`content_inspector` crate guesses binary vs text. Can misclassify. No way for authors to explicitly mark a file as "don't template this." Large files are read into memory entirely for detection, then again for rendering.

**v3 fix:** Support `.archetect-ignore` or manifest-level file rules:
```yaml
templating:
  skip:
    - "*.png"
    - "*.jar"
    - "vendor/**"
```

### No encoding validation

`fs::read_to_string(path)?` on a Latin-1 file produces `"Error: File I/O error"` with no indication of the encoding issue.

**v3 fix:** Detect encoding issues and provide specific error: "file.txt contains invalid UTF-8. Templates must be UTF-8. Place binary files in a non-template directory or add to skip list."

### Template rendering errors lack source context

When a template fails to render, the error doesn't include which template variable was missing or what the expression was. Users get a generic render error and have to guess which `{{ }}` expression failed.

**v3 fix:** Include the template filename, the failing expression, and the available context keys in the error message.

---

## Low: Nice-to-Have Improvements

### Watch mode for authoring
`archetect watch <archetype> <destination> --answers answers.yaml` that re-renders on file changes. Dramatically speeds up the edit-render-check cycle.

### Answer file generation
`archetect answers record` that renders interactively and records all prompt responses into a YAML file. Useful for creating CI/CD answer files and test fixtures.

### Answer file validation
Validate that an answer file covers all required prompts for an archetype before rendering. Catch missing answers early instead of failing mid-render.

### Archetype scaffold command
`archetect init` that generates a minimal archetype skeleton (archetype.yaml, archetype.lua, contents/) so authors don't start from scratch.

### Render progress
Show a progress indicator for long renders, especially when fetching git sources or rendering large directory trees.

---

## Summary: v3 Priority Matrix

### Must Fix (blocking quality issues)
1. Replace all `.unwrap()` / `.expect()` with proper error handling
2. Rich script error reporting with line numbers and file paths
3. Manifest validation at load time
4. Dry-run mode
5. Archetype testing framework (`archetect test`)
6. Component version pinning

### Should Fix (significant UX improvements)
7. Atomic rendering with rollback
8. Conflict diff view
9. Cache transparency
10. Config merge visibility
11. Explicit file type markers (skip-template rules)
12. `archetect list` for catalog browsing
13. Complete documentation (templating reference, API docs, changelog)

### Nice to Have (polish)
14. Watch mode for authoring
15. REPL for interactive scripting
16. Answer file generation and validation
17. Archetype scaffold command
18. Render progress indicator

---

## Relationship to Other v3 Plans

- [archetect-3-io-overhaul.md](archetect-3-io-overhaul.md) — The IO protocol overhaul enables dry-run (capture WriteFile messages), atomic rendering (batch writes), and conflict diffs (compare before writing). Many fixes here depend on that overhaul.
- [archetect-3-lua-scripting-engine.md](archetect-3-lua-scripting-engine.md) — Lua's Context object can enforce strict property access (fixing silent typo failures). LuaLS annotations provide the API documentation that's currently missing. The REPL is trivial with Lua. The redesigned API addresses the verbose/confusing casing system and overloaded function signatures.
