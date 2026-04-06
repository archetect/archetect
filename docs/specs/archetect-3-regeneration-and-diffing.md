# Archetect 3: Project Regeneration and Diff-Aware Writes

## Status

**Draft** -- design spec, not yet implemented.

## Problem

Archetect generates projects from archetypes, but today every render is a one-shot operation. Once a user modifies the generated project, re-running the archetype overwrites their changes. This makes it impossible to adopt archetype improvements (new CI config, updated dependencies, structural enhancements) without manually replaying diffs.

Users have asked for this capability for years. The question has always been: *how do you merge generated output with user modifications without destroying either?*

## Scope and Applicability

**Regeneration is an opt-in capability, not a universal requirement.** Archetypes serve many different workflows, and most don't need regeneration at all:

- **One-shot generation** remains the default and most common pattern. Generate a project, move on. No manifest, no shadow directory, no overhead. This is how the vast majority of archetypes work today and will continue to work.

- **Additive workflows** are a distinct pattern where users generate a project, then `cd` into it and run *different* archetypes to add entities, API endpoints, database migrations, etc. These incremental archetypes typically append or create new files rather than modifying existing ones. They don't need regeneration -- they need clean `generate_once` and `overwrite` semantics, which already exist.

- **Managed project workflows** are the niche where regeneration shines. A platform team maintains an archetype that defines CI, build config, Docker setup, and project structure. Teams generate from it, then customize. When the platform archetype evolves (new CI pipeline, updated base images, security patches), teams regenerate to pull in the improvements without losing their customizations.

Regeneration support is declared per-archetype in `archetype.yaml`. Archetypes without a `regeneration` section behave exactly as they do today -- pure one-shot generation with no additional state or overhead. The framework provides the primitives; archetype authors choose which workflow their archetype supports.

## Design Principles

1. **Leverage what exists.** Archetect already loads `.archetect.yaml` from the project directory for answer persistence. Build on that, don't replace it.
2. **Opt-in, not default.** Regeneration adds complexity (manifest, shadow directory, merge logic). Only archetypes that declare `regeneration` in their manifest participate. One-shot and additive archetypes are unaffected.
3. **Archetype authors control write strategy.** The archetype knows which files are scaffolding (generate once), which are fully owned by the archetype (overwrite freely), and which are mixed. Expose that as a first-class manifest concept.
4. **Degrade gracefully.** If there's no prior generation state, regeneration still works -- it just can't three-way merge. Files get the same overwrite/preserve behavior they do today.
5. **The IO channel is the integration point.** File writes already flow through `ScriptIoHandle` as `WriteFile` messages. Write strategies and merge logic plug into the handler, not the scripting engine.
6. **Don't require git.** Many projects use git, but the regeneration system must be self-contained. Git-aware optimizations are optional enhancements, not requirements.

## Concepts

### Generation Manifest

A `.archetect/manifest.yaml` file stored in the generated project. Created or updated on every render. Contains:

```yaml
# .archetect/manifest.yaml
archetype:
  source: "https://github.com/archetect/archetype-rust-cli.git"
  ref: "v2.3.0"                    # Tag, branch, or commit used
  checksum: "sha256:abc123..."     # Integrity check of archetype at render time

generated_at: "2026-04-05T14:30:00Z"
archetect_version: "3.0.0"

answers:
  project_name: "my-service"
  author_name: "Jimmie Fulton"
  include_ci: true
  # ... all answers given during this render

switches:
  - include_tests
  - include_docs

files:
  "Cargo.toml":
    checksum: "sha256:def456..."
    strategy: merge
  "src/main.rs":
    checksum: "sha256:789abc..."
    strategy: generate_once
  ".github/workflows/build.yml":
    checksum: "sha256:aaa111..."
    strategy: overwrite
  "src/generated/models.rs":
    checksum: "sha256:bbb222..."
    strategy: overwrite
```

The `files` section records the checksum of every file *as generated*. This is the key to detecting user modifications without a shadow directory.

### Shadow Directory

`.archetect/shadow/` stores a complete copy of the last-generated output. This enables three-way merge when a file has been modified by both the user and the archetype.

The shadow directory is an implementation detail -- users should `.gitignore` or ignore it. It can be regenerated from the archetype + manifest answers at any time (it's a cache, not source of truth).

**Size concern:** For most projects the shadow directory is small (source templates expand to kilobytes, not megabytes). For archetypes that generate large binary assets, the `overwrite` strategy avoids needing a shadow copy of those files.

### Write Strategies

Each file produced by an archetype has a **write strategy** that governs how it's handled during regeneration. Strategies are resolved in this order:

1. Explicit per-file strategy in `archetype.yaml` (archetype author's intent)
2. Glob-pattern rules in `archetype.yaml` (e.g., `src/generated/**` -> `overwrite`)
3. Default strategy (configurable in manifest, defaults to `merge`)

#### Strategy Definitions

| Strategy | First Generate | Regenerate (unmodified) | Regenerate (user modified) |
|----------|---------------|------------------------|---------------------------|
| `overwrite` | Write | Overwrite | Overwrite (user changes lost) |
| `generate_once` | Write | Skip | Skip |
| `merge` | Write | Overwrite (safe, no user changes) | Three-way merge |
| `protected_regions` | Write | Replace marked regions only | Replace marked regions only |
| `prompt` | Write | Ask user | Ask user |

#### `overwrite`

The archetype fully owns this file. Every regeneration replaces it unconditionally. Appropriate for:
- CI/CD configuration
- Generated code (models, schemas, bindings)
- Docker/build files the archetype manages

#### `generate_once`

The file is scaffolding -- created on first render, then owned entirely by the user. Regeneration never touches it. Appropriate for:
- `src/main.rs` or application entry points
- README.md
- Any file the user is expected to heavily customize

#### `merge`

The default strategy. Uses three-way merge to incorporate changes from both the user and the archetype:

1. **If user hasn't modified the file** (checksum matches manifest): overwrite safely.
2. **If archetype hasn't changed the file** (new generated output matches shadow): skip, nothing to do.
3. **If both changed, non-overlapping regions**: auto-merge.
4. **If both changed, overlapping regions**: write conflict markers and report to user.

Requires the shadow directory for the base version. If no shadow exists (first regeneration of a legacy project), falls back to `prompt`.

#### `protected_regions`

For files where generated and user code coexist. The archetype marks owned regions with delimiters:

```rust
// archetect:begin:imports
use actix_web::{web, App, HttpServer};
use crate::routes::health;
// archetect:end:imports

// User adds their own imports here -- untouched by regeneration

fn main() {
    // archetect:begin:server_setup
    let server = HttpServer::new(|| {
        App::new()
            .route("/health", web::get().to(health::check))
    });
    // archetect:end:server_setup

    // User customization below -- untouched
    server.bind("0.0.0.0:8080").unwrap().run().unwrap();
}
```

On regeneration, only content between matching `archetect:begin:<name>` / `archetect:end:<name>` pairs is replaced. Everything else is preserved verbatim. Region delimiters use the file's comment syntax (detected from file extension or specified in manifest).

#### `prompt`

Ask the user what to do. Presents a diff and offers choices: overwrite, skip, merge (if shadow available), or open in editor.

### Strategy Declaration in `archetype.yaml`

Archetype authors declare strategies in the manifest:

```yaml
# archetype.yaml
regeneration:
  default_strategy: merge

  strategies:
    # Exact file paths
    - path: "src/main.rs"
      strategy: generate_once

    - path: "Cargo.toml"
      strategy: merge

    # Glob patterns (evaluated in order, first match wins)
    - path: ".github/**"
      strategy: overwrite

    - path: "src/generated/**"
      strategy: overwrite

    - path: "*.lock"
      strategy: generate_once

    - path: "docker-compose.yml"
      strategy: protected_regions

    # Script-authored files can also specify strategy inline
    # (see "Script Integration" below)
```

If no `regeneration` section exists, the archetype is a standard one-shot archetype. All files use today's `overwrite`/`preserve` behavior. No manifest is recorded, no shadow directory is created. This is the expected case for most archetypes -- entity generators, API scaffolders, utility archetypes, and anything designed for additive workflows.

## Architecture

### Where It Fits

```
                        archetype.rhai / archetype.lua
                                    |
                          render_directory() / render()
                                    |
                    ScriptMessage::WriteFile(WriteFileInfo)
                                    |
                        +-----------+-----------+
                        |                       |
                  First Render            Regeneration
                        |                       |
                   Write file            Resolve strategy
                   Record in manifest    Compare checksums
                   Copy to shadow        Three-way merge (if needed)
                        |                Write / skip / conflict
                        |                Update manifest + shadow
                        |                       |
                        +-----------+-----------+
                                    |
                          ClientMessage::Ack / Error
```

### WriteFileInfo Extension

The existing `WriteFileInfo` in `archetect-api` gains an optional strategy field:

```rust
pub struct WriteFileInfo {
    pub destination: String,
    pub contents: Vec<u8>,
    pub write_strategy: WriteStrategy,
}

pub enum WriteStrategy {
    /// Use the strategy from the archetype manifest, or fall back to default.
    /// This is what render_directory() emits for template-generated files.
    FromManifest,
    /// Explicit strategy set by the script (e.g., for dynamically generated files).
    Explicit(FileStrategy),
    /// Legacy behavior: overwrite or preserve based on existing_file_policy.
    /// Used by v2 Rhai scripts that haven't opted into regeneration.
    Legacy(ExistingFilePolicy),
}

pub enum FileStrategy {
    Overwrite,
    GenerateOnce,
    Merge,
    ProtectedRegions,
    Prompt,
}
```

The `Legacy` variant ensures backwards compatibility: existing Rhai scripts that use `overwrite()` or `preserve()` continue to work unchanged.

### Regeneration Handler

The write handler in the IO driver (terminal, server, etc.) implements the regeneration logic:

```
fn handle_write_file(info: WriteFileInfo, manifest: &mut Manifest, shadow: &Path) -> Result<WriteOutcome> {
    let strategy = resolve_strategy(&info, manifest);
    let dest = Path::new(&info.destination);

    match strategy {
        Overwrite => {
            write_file(dest, &info.contents)?;
            update_manifest_and_shadow(manifest, shadow, &info)?;
            Ok(WriteOutcome::Written)
        }
        GenerateOnce => {
            if dest.exists() {
                Ok(WriteOutcome::Skipped)
            } else {
                write_file(dest, &info.contents)?;
                update_manifest_and_shadow(manifest, shadow, &info)?;
                Ok(WriteOutcome::Written)
            }
        }
        Merge => {
            handle_merge(dest, &info.contents, manifest, shadow)
        }
        ProtectedRegions => {
            handle_protected_regions(dest, &info.contents)
        }
        Prompt => {
            // Send prompt to user, wait for decision
            handle_interactive_decision(dest, &info.contents, manifest, shadow)
        }
    }
}
```

### Three-Way Merge Flow

```
fn handle_merge(dest: &Path, new_contents: &[u8], manifest: &Manifest, shadow: &Path) -> Result<WriteOutcome> {
    if !dest.exists() {
        // New file, just write it
        return write_and_record(dest, new_contents, manifest, shadow);
    }

    let current = fs::read(dest)?;
    let current_checksum = sha256(&current);
    let manifest_checksum = manifest.file_checksum(dest);

    if Some(current_checksum) == manifest_checksum {
        // User hasn't modified the file -- safe to overwrite
        return write_and_record(dest, new_contents, manifest, shadow);
    }

    // User has modified the file. Do we have a shadow (base) for three-way merge?
    let shadow_path = shadow.join(relative_path(dest));
    if shadow_path.exists() {
        let base = fs::read(&shadow_path)?;

        if base == new_contents {
            // Archetype hasn't changed this file -- keep user's version
            return Ok(WriteOutcome::Skipped);
        }

        // Both sides changed -- three-way merge
        match three_way_merge(&base, &current, new_contents) {
            MergeResult::Clean(merged) => {
                write_and_record(dest, &merged, manifest, shadow)?;
                Ok(WriteOutcome::Merged)
            }
            MergeResult::Conflict(merged_with_markers) => {
                fs::write(dest, &merged_with_markers)?;
                Ok(WriteOutcome::Conflict)
            }
        }
    } else {
        // No shadow available -- can't three-way merge
        // Fall back to prompting the user
        handle_interactive_decision(dest, new_contents, manifest, shadow)
    }
}
```

### Protected Regions Flow

```
fn handle_protected_regions(dest: &Path, new_contents: &[u8]) -> Result<WriteOutcome> {
    if !dest.exists() {
        return write_file(dest, new_contents).map(|_| WriteOutcome::Written);
    }

    let current = String::from_utf8_lossy(&fs::read(dest)?);
    let generated = String::from_utf8_lossy(new_contents);

    let merged = replace_regions(&current, &generated)?;
    fs::write(dest, merged.as_bytes())?;
    Ok(WriteOutcome::RegionsUpdated)
}

fn replace_regions(current: &str, generated: &str) -> Result<String> {
    // Parse generated content to extract named regions
    let new_regions = parse_regions(generated)?;

    // Walk current content, replacing region contents while preserving everything else
    let mut output = String::new();
    let mut in_region: Option<&str> = None;
    let mut skip_until_end = false;

    for line in current.lines() {
        if let Some(name) = parse_region_begin(line) {
            output.push_str(line);
            output.push('\n');
            in_region = Some(name);
            skip_until_end = true;
            // Write new region content
            if let Some(new_content) = new_regions.get(name) {
                output.push_str(new_content);
            }
        } else if parse_region_end(line).is_some() {
            skip_until_end = false;
            in_region = None;
            output.push_str(line);
            output.push('\n');
        } else if !skip_until_end {
            output.push_str(line);
            output.push('\n');
        }
    }

    Ok(output)
}
```

## CLI Interface

### Regeneration Command

```bash
# Regenerate the current project from its archetype
archetect regenerate

# Regenerate with updated answers (prompts for new/changed questions)
archetect regenerate --interactive

# Regenerate from a different archetype version
archetect regenerate --ref v3.0.0

# Dry run -- show what would change without writing
archetect regenerate --dry-run

# Force overwrite all files (ignore strategies)
archetect regenerate --force
```

The `regenerate` command:

1. Reads `.archetect/manifest.yaml` from the current directory
2. Resolves the archetype source (pulling updates if needed)
3. Loads answers from the manifest, merged with `.archetect.yaml` (project-local config takes precedence)
4. If archetype has new prompts not in the manifest, prompts the user (or uses defaults in headless mode)
5. Renders to a temp directory
6. Applies write strategies file-by-file against the existing project
7. Updates manifest and shadow
8. Reports summary: written, skipped, merged, conflicted

### Render with Manifest Recording

Manifest recording is **automatic** when the archetype declares a `regeneration` section in `archetype.yaml`. No CLI flag needed -- the archetype author's declaration is the opt-in.

```bash
# Render a regeneration-enabled archetype -- manifest is recorded automatically
archetect render https://github.com/org/platform-archetype.git ./my-project

# Render a standard one-shot archetype -- no manifest, no overhead
archetect render https://github.com/org/entity-archetype.git ./my-project
```

When manifest recording is active, the render:
- Creates `.archetect/manifest.yaml` with archetype source, answers, and file checksums
- Creates `.archetect/shadow/` with copies of all generated files
- Adds `.archetect/shadow/` to `.gitignore` (if git repo detected)

### Output Report

After regeneration, display a summary:

```
Regeneration complete.

  Written:    12 files (archetype changes, no user modifications)
  Skipped:     5 files (generate_once, or archetype unchanged)
  Merged:      3 files (user + archetype changes combined cleanly)
  Conflicted:  1 file  (manual resolution needed)

Conflicts:
  src/lib.rs -- resolve conflict markers, then run: archetect resolve src/lib.rs

Updated: .archetect/manifest.yaml
```

## Script Integration

### Rhai (v2 Compatibility)

Existing Rhai scripts work unchanged. The `render_directory()` and `render()` functions continue to emit `WriteFile` messages with `Legacy(ExistingFilePolicy)`. Regeneration strategies only apply if the archetype manifest declares a `regeneration` section.

For Rhai archetypes that want regeneration support, the manifest is sufficient -- no script changes needed. The strategy is declared in `archetype.yaml`, not in the script.

### Lua (v3 API)

The Lua API can set strategies programmatically for dynamically generated files:

```lua
-- Render a directory with default strategies from manifest
ctx:render_directory("contents", destination)

-- Write a single file with explicit strategy
ctx:write_file("src/generated/schema.rs", content, {
    strategy = "overwrite"
})

-- Write scaffolding that the user will own
ctx:write_file("src/main.rs", content, {
    strategy = "generate_once"
})
```

## Answer Evolution

When an archetype adds new prompts in a newer version:

1. Manifest contains answers from the previous render
2. New prompts have no answer in the manifest
3. Behavior depends on mode:
   - **Interactive**: prompt the user for the new questions only
   - **Headless / `--use-defaults-all`**: use the prompt's default value
   - **`--answer` CLI flag**: can supply new answers on the command line

When an archetype *removes* a prompt, the stale answer in the manifest is harmless -- it's simply unused. The updated manifest will no longer include it.

When an archetype *renames* a prompt, the old answer is lost. Archetype authors should document migrations. A future enhancement could support answer aliases in the manifest.

## Interaction with IO Overhaul

This spec builds directly on the IO overhaul (Phase 2):

- **WriteFile messages already flow through the IO channel.** The regeneration handler is an enhancement to the write handler, not a separate path.
- **The `Ack`/`Error` response protocol handles write outcomes.** Extending `Ack` to carry a `WriteOutcome` variant (written, skipped, merged, conflicted) gives the script visibility into what happened.
- **The gRPC server (Phase 4) gets regeneration for free.** Since the logic lives in the IO handler, any client (CLI, CodegenExtension, IDE plugin) benefits.

## Interaction with `.archetect.yaml`

The existing `.archetect.yaml` project-local config and the new `.archetect/manifest.yaml` serve complementary purposes:

| | `.archetect.yaml` | `.archetect/manifest.yaml` |
|---|---|---|
| **Purpose** | User-facing project configuration | Machine-generated render state |
| **Who writes it** | User (or archetype on first render) | Archetect (automatically) |
| **Checked into git** | Yes | Yes (except shadow/) |
| **Contains answers** | User's preferred defaults | Exact answers from last render |
| **Precedence** | Higher (user intent overrides) | Lower (baseline for comparison) |

During regeneration, answers merge as: `manifest answers` < `.archetect.yaml` answers < `--answer` CLI flags. This means a user can change an answer in `.archetect.yaml` and regenerate -- the new answer takes effect, and the manifest updates to reflect it.

## Implementation Phases

### Phase 1: Manifest Recording

- Parse `regeneration` section from `archetype.yaml`
- When present, automatically generate `.archetect/manifest.yaml` on render (archetype source, answers, file checksums)
- No shadow directory yet, no regeneration command yet
- This is the minimum to start building state for future regeneration
- Archetypes without `regeneration` section are completely unaffected

### Phase 2: Shadow Directory + Regenerate Command

- Create `.archetect/shadow/` during recorded renders
- Implement `archetect regenerate` command
- Implement `overwrite`, `generate_once`, and `prompt` strategies
- Checksum-based "unmodified file" detection for safe overwrites

### Phase 3: Three-Way Merge

- Implement `merge` strategy using diff3 algorithm
- Conflict marker generation and reporting
- `archetect resolve` helper command (or integrate with user's merge tool)

### Phase 4: Protected Regions

- Region parser (language-aware comment detection)
- Region replacement logic
- `protected_regions` strategy in write handler

### Phase 5: Dry Run + Polish

- `--dry-run` flag showing planned changes without writing
- Improved conflict reporting and resolution UX

## Open Questions

1. **Shadow directory vs. reconstruct-on-demand?** The shadow directory is simple but costs disk space. Alternative: re-render the archetype at the recorded version with recorded answers to reconstruct the base. More complex, requires the old archetype version to be available.

2. **Conflict marker format?** Git-style `<<<<<<<` / `=======` / `>>>>>>>` is universally understood. But custom markers (e.g., `archetect:conflict`) could carry more metadata. Recommend git-style for tool compatibility.

3. **Component archetypes?** When an archetype composes child archetypes via `components`, each child needs its own manifest section. The manifest should support nested archetype tracking.

4. **Binary files?** Three-way merge doesn't work for binary files. Binary files should default to `overwrite` or `prompt`. The strategy resolution should detect binary content and refuse to merge.

5. **Interaction with additive archetypes?** A project might be initially generated from a regeneration-enabled archetype, then have entities/APIs added by one-shot archetypes. The additive archetypes should not interfere with the manifest -- they create new files that the manifest doesn't track. On regeneration, untracked files (created by other archetypes or the user) are left untouched. This should "just work" but needs validation with real workflows.

## Dependencies

- **IO Overhaul Phase 2** (in progress): WriteFile messages through IO channel
- **Rust crates**: `similar` or `diffy` for three-way merge, `sha2` for checksums
- **No external tools required**: The merge algorithm is embedded, not shelled out to `git merge-file`
