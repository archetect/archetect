# Unwrap Hardening & ATL Template Engine Audit

## Session Goal

Eliminate panic paths in production code and harden the ATL (Archetect Template Language) engine for robustness. This is a code-quality session — no new features, just making what we have bulletproof.

## Part 1: Unwrap Hardening

### Inventory

After excluding test code, vendored crates (`archetect-templating`, `archetect-terminal-prompts`), and the `vendor/` directory, there are **19 production `.unwrap()` calls** across the workspace.

### By Risk Level

#### Safe — Leave As-Is (7)
These unwrap on compile-time constants or hardcoded patterns that cannot fail:

| File | Line | Expression | Why Safe |
|------|------|-----------|----------|
| `archetect-core/src/archetect/archetect.rs` | 83 | `Version::parse(env!("CARGO_PKG_VERSION")).unwrap()` | Cargo guarantees valid semver |
| `archetect-core/src/archetype/archetype_manifest/requirements.rs` | 38 | `VersionReq::parse(env!("CARGO_PKG_VERSION")).unwrap()` | Cargo guarantees valid semver |
| `archetect-core/src/source.rs` | 147 | `Regex::new(r"\S+@(\S+):(.*)").unwrap()` | Hardcoded pattern, always valid |
| `archetect-inflections/src/string/pluralize.rs` | 31 | `Regex::new(rule).unwrap()` | Static patterns in lazy_static |
| `archetect-inflections/src/string/singularize.rs` | 141 | `Regex::new(rule).unwrap()` | Static patterns in lazy_static |
| `archetect-bin/src/main.rs` | 254 | `matches.get_one::<String>("source").unwrap()` | Clap enforces required arg |
| `archetect-bin/src/cli.rs` | 355 | Logger init unwrap | Infallible in practice |

**Action**: Convert to `.expect("reason")` for documentation, or leave as-is.

#### Guarded — Low Risk, Worth Hardening (6)
These have safety guards but are technically fragile:

| File | Line | Expression | Guard | Suggested Fix |
|------|------|-----------|-------|---------------|
| `archetect-core/src/source.rs` | 187 | `url.host_str().unwrap()` | After `url.has_host()` check | Use `.ok_or(SourceError::...)` |
| `archetect-core/src/source.rs` | 287 | `timestamp.unwrap()` | After chrono parse | Use `?` or `.ok_or()` |
| `archetect-core/src/source.rs` | 333 | `gitref.as_ref().unwrap()` | After `is_some()` check | Pattern match instead |
| `archetect-bin/src/configuration/mod.rs` | 46 | `found.into_iter().next().unwrap()` | After `found.len() == 1` | Pattern match |
| `archetect-bin/src/configuration/mod.rs` | 50 | `p.file_name().unwrap()` | Valid PathBuf | Use `.ok_or()` |
| `archetect-core/src/source.rs` | 440 | `String::from_utf8(...).unwrap_or(...)` | Has fallback | Already safe (unwrap_or) |

#### Must Fix — Real Panic Risk (4)

| File | Line | Expression | Issue | Fix |
|------|------|-----------|-------|-----|
| `archetect-core/src/source.rs` | 317 | `cached_paths().lock().unwrap()` | Mutex poison panic | `.expect("cache lock")` or handle poison |
| `archetect-core/src/source.rs` | 348 | `cached_paths().lock().unwrap()` | Mutex poison panic | Same |
| `archetect-core/src/script/lua/template_engine/render.rs` | 132 | `Utf8PathBuf::from_path_buf(entry.path()).unwrap()` | Non-UTF-8 path panic | `.map_err()` with new `RenderError` variant |
| `archetect-mcp/src/session.rs` | 100, 108 | `PromptEnvelope::from_script_message(msg).unwrap()` | Unexpected message type | Return `Err()` instead of panic |

### Approach

1. Fix the 4 "must fix" items with proper error propagation
2. Harden the 6 "guarded" items by replacing guard+unwrap with pattern matching
3. Add `.expect("reason")` to the 7 "safe" items for documentation
4. Add a `#[deny(clippy::unwrap_used)]` lint to catch future unwraps (with `#[allow]` on the justified safe ones)

## Part 2: ATL Template Engine Audit

### Architecture (1326 lines total)

```
Template String
    ↓ tokenizer.rs (391 lines)
Vec<Token> — Text, Expression, Logic, Comment
    ↓ compiler.rs (404 lines)
Lua Function Source (String)
    ↓ render.rs (219 lines) — mlua VM
Rendered Output (String)
```

### Current State

**Strengths:**
- Clean tokenize → compile → render separation
- All mlua operations in render.rs have proper `map_err()` error handling
- Line-accurate error reporting from tokenizer
- Kebab-case key support via bracket notation
- Comprehensive compile error types (unterminated blocks, empty expressions, invalid filters)
- No TODO/FIXME comments
- No unsafe code

**Issues to Address:**

| Issue | Severity | Location | Description |
|-------|----------|----------|-------------|
| **UTF-8 path panic** | HIGH | render.rs:132 | `from_path_buf().unwrap()` — panics on non-UTF-8 paths |
| **Unbounded template cache** | MEDIUM | render.rs:18 | `HashMap` with no size limit or eviction |
| **Missing variable behavior** | MEDIUM | Generated Lua | Undefined vars silently become `nil` — may produce confusing output |
| **Deferred Lua syntax errors** | MEDIUM | compiler.rs | Malformed Lua in `{%...%}` blocks accepted by tokenizer, fails at runtime |
| **No filter validation** | LOW | render.rs | Filters passed as Lua table — no check that they're callable |

### Test Coverage

**Well tested:** Tokenizer (all delimiter types, edge cases, error paths), compiler (escaping, filters, identifiers), integration (interpolation, loops, conditionals, nested iteration, proto3 template).

**Gaps:**
- No tests for missing context variables
- No tests for filter runtime failures
- No tests for directory recursion (only integration-tested)
- No tests for non-UTF-8 path handling
- No stress tests (very large templates)

### Recommended Actions

1. **Fix the UTF-8 panic** — add `RenderError::InvalidUtf8Path` variant
2. **Add cache bounds** — either document the assumption (templates are finite per archetype) or add a soft limit
3. **Add error-case tests** — missing vars, failing filters, invalid Lua in logic blocks
4. **Consider Lua syntax pre-validation** — after compilation, try `lua.load(source).into_function()` and return compile error if it fails, rather than deferring to render-time

## Files to Touch

### Must modify
- `archetect-core/src/source.rs` — unwrap hardening (mutex, URL, gitref)
- `archetect-core/src/script/lua/template_engine/render.rs` — UTF-8 path fix, cache bounds
- `archetect-core/src/script/lua/template_engine/error.rs` — new error variant
- `archetect-mcp/src/session.rs` — message unwrap → error
- `archetect-bin/src/configuration/mod.rs` — guard → pattern match

### Should modify
- `archetect-core/src/script/lua/template_engine/mod.rs` — new tests
- `archetect-core/src/script/lua/template_engine/compiler.rs` — Lua syntax pre-check

### Optional
- Workspace `Cargo.toml` or `clippy.toml` — `#[deny(clippy::unwrap_used)]`

## Verification

```bash
cargo build                    # 0 warnings
cargo test                     # all pass
cargo clippy --all-targets     # no unwrap warnings (if lint added)
```
