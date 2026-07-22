//! Runtime introspection of the scripting API — the LuaCATS annotation stubs, parsed into
//! structured entries an **agent** can ask for (`archetect introspect`, MCP `introspect`).
//!
//! **One source, two sinks.** The stubs in `lua/annotations/` already ship to an author's editor
//! (`archetect ide setup` installs them). They are hand-written, rich, and canonical — so this
//! module makes them the source for a second sink: runtime answers about the API's shape, without
//! opening archetect's source. Deriving from the stub rather than a parallel Rust registry is
//! deliberate: a registry would be a second place to write the same summary, and the two would
//! drift. The stub is what ships to the editor, so it is the copy that cannot be allowed to rot.
//!
//! Ported from prova's `help.rs` — the two projects share the LuaCATS dialect (and will share the
//! parser as a crate once both ports stabilize; see docs/plans/autodidact.md §4.2).

/// The core LuaCATS stubs, embedded once and consumed twice: here (→ introspection) and by
/// archetect-bin's `ide setup` (→ the IDE annotation folder).
pub const CORE_STUBS: &[(&str, &str)] = &[
    ("archetect.lua", include_str!("../lua/annotations/archetect.lua")),
    (
        "archetect_modules.lua",
        include_str!("../lua/annotations/archetect_modules.lua"),
    ),
];

/// One documented thing: a function, a method, or a class (a value shape).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelpEntry {
    /// `template.render`, `Context:prompt_text`, `GitRepo`.
    pub name: String,
    /// `(message: string, key: string, opts?: table) -> string|nil`, or for a class its field
    /// shape `{ created: boolean, empty: boolean }`.
    pub signature: String,
    /// The stub's prose, collapsed to one line.
    pub summary: String,
}

/// Split a LuaCATS type off its trailing note — either a `# note` comment or, in this repo's
/// stub dialect, free prose after the first type token (`---@param key string Key to retrieve`).
fn split_note(s: &str) -> (String, Option<String>) {
    let (head, hash_note) = match s.split_once('#') {
        Some((ty, note)) => {
            let note = note.trim();
            (ty, (!note.is_empty()).then(|| note.to_string()))
        }
        None => (s, None),
    };
    let mut it = head.trim().splitn(2, char::is_whitespace);
    let ty = it.next().unwrap_or("").to_string();
    let trailing = it
        .next()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(String::from);
    (ty, hash_note.or(trailing))
}

/// Collapse accumulated `---` prose lines into one summary line.
fn collapse(prose: &[String]) -> String {
    prose
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Parse one `---@meta` LuaCATS stub into help entries.
///
/// Recognises the shapes the stubs actually use: `--- prose`, `---@param n ty`, `---@return ty`,
/// `---@class Name`, `---@field n ty`, and the `function name(args) end` / `function C:m() end`
/// declarations they document. Anything else is ignored — this reads documentation, it does not
/// type-check Lua.
pub fn parse_stub(src: &str) -> Vec<HelpEntry> {
    let mut out = Vec::new();
    let mut prose: Vec<String> = Vec::new();
    let mut params: Vec<(String, String)> = Vec::new();
    let mut ret: Option<String> = None;
    // A class stays open across its `---@field` lines and flushes when the block ends:
    // (name, summary, fields as (name, type, note)).
    type OpenClass = (String, String, Vec<(String, String, Option<String>)>);
    let mut class: Option<OpenClass> = None;

    let flush_class = |class: &mut Option<OpenClass>, out: &mut Vec<HelpEntry>| {
        if let Some((name, summary, fields)) = class.take() {
            let body = fields
                .iter()
                .map(|(n, ty, note)| match note {
                    Some(note) => format!("{n}: {ty}  -- {note}"),
                    None => format!("{n}: {ty}"),
                })
                .collect::<Vec<_>>()
                .join(", ");
            out.push(HelpEntry {
                name,
                signature: if body.is_empty() {
                    "{}".into()
                } else {
                    format!("{{ {body} }}")
                },
                summary,
            });
        }
    };

    for line in src.lines() {
        let t = line.trim();

        if let Some(rest) = t.strip_prefix("---@class ") {
            flush_class(&mut class, &mut out);
            let name = rest.split_whitespace().next().unwrap_or("").to_string();
            class = Some((name, collapse(&prose), Vec::new()));
            prose.clear();
            continue;
        }
        if let Some(rest) = t.strip_prefix("---@field ") {
            if let Some((n, ty)) = rest.split_once(char::is_whitespace) {
                let (ty, note) = split_note(ty);
                if let Some((_, _, fields)) = class.as_mut() {
                    fields.push((n.trim().to_string(), ty, note));
                }
            }
            continue;
        }
        if let Some(rest) = t.strip_prefix("---@param ") {
            flush_class(&mut class, &mut out);
            if let Some((n, ty)) = rest.split_once(char::is_whitespace) {
                params.push((n.trim().to_string(), split_note(ty).0));
            }
            continue;
        }
        if let Some(rest) = t.strip_prefix("---@return ") {
            flush_class(&mut class, &mut out);
            ret = Some(split_note(rest).0);
            continue;
        }
        // Prose: `--- text` / `---text` (but not another `---@tag` we don't model). Prose after
        // an open `---@class` belongs to the class (this repo's dialect documents classes below
        // the tag), everything else accumulates for the next declaration.
        if let Some(rest) = t.strip_prefix("---") {
            if !rest.starts_with('@') {
                let text = rest.trim();
                if !text.is_empty() {
                    match class.as_mut() {
                        Some((_, summary, fields)) if fields.is_empty() => {
                            if !summary.is_empty() {
                                summary.push(' ');
                            }
                            summary.push_str(text);
                        }
                        _ => prose.push(text.to_string()),
                    }
                }
            }
            continue; // an unmodelled tag — ignore, don't let it leak into prose
        }
        // A declaration closes the block: `function template.render(source, context, opts) end`.
        if let Some(rest) = t.strip_prefix("function ") {
            flush_class(&mut class, &mut out);
            if let Some((name, _)) = rest.split_once('(') {
                let name = name.trim().to_string();
                let args = params
                    .iter()
                    .map(|(n, ty)| format!("{n}: {ty}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                let sig = match &ret {
                    Some(r) => format!("({args}) -> {r}"),
                    None => format!("({args})"),
                };
                if !name.is_empty() {
                    out.push(HelpEntry {
                        name,
                        signature: sig,
                        summary: collapse(&prose),
                    });
                }
            }
            prose.clear();
            params.clear();
            ret = None;
            continue;
        }
        // Any other line ends an open block (e.g. `local GitRepo = {}` after a class).
        if !t.is_empty() {
            flush_class(&mut class, &mut out);
        }
        if t.is_empty() {
            prose.clear();
            params.clear();
            ret = None;
        }
    }
    flush_class(&mut class, &mut out);
    out
}

/// Every entry from the embedded core stubs, sorted by name.
pub fn core_entries() -> Vec<HelpEntry> {
    let mut out: Vec<HelpEntry> = CORE_STUBS
        .iter()
        .flat_map(|(_, src)| parse_stub(src))
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out.dedup_by(|a, b| a.name == b.name);
    out
}

/// Case-insensitive substring match across name and summary — `introspect prompt`,
/// `introspect case`.
pub fn filter(entries: &[HelpEntry], needle: &str) -> Vec<HelpEntry> {
    let n = needle.to_lowercase();
    entries
        .iter()
        .filter(|e| e.name.to_lowercase().contains(&n) || e.summary.to_lowercase().contains(&n))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_documented_function() {
        let entries = parse_stub(
            r#"
--- Render a template string against a context, returning the result (or writing it when
--- opts.destination is set).
---@param template string
---@param context Context|table
---@param opts? table
---@return string
function template.render(template, context, opts) end
"#,
        );
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "template.render");
        assert_eq!(
            entries[0].signature,
            "(template: string, context: Context|table, opts?: table) -> string"
        );
        assert!(entries[0].summary.starts_with("Render a template string"));
        assert!(!entries[0].summary.contains('\n'));
    }

    #[test]
    fn parses_a_class_into_its_field_shape() {
        let entries = parse_stub(
            r#"
--- The result of github.create_repo.
---@class CreateRepoResult
---@field created boolean
---@field empty boolean    # true when the repo has no commits
local CreateRepoResult = {}
"#,
        );
        let c = entries
            .iter()
            .find(|e| e.name == "CreateRepoResult")
            .expect("class entry");
        assert_eq!(
            c.signature,
            "{ created: boolean, empty: boolean  -- true when the repo has no commits }"
        );
        assert_eq!(c.summary, "The result of github.create_repo.");
    }

    /// The real embedded stubs parse into a substantial surface, covering the calls an agent
    /// authoring an archetype reaches for first.
    #[test]
    fn the_real_stubs_cover_the_authoring_surface() {
        let all = core_entries();
        assert!(
            all.len() > 40,
            "expected a substantial surface, got {}",
            all.len()
        );
        for needle in ["prompt_text", "prompt_select", "catalog.render", "shell.run"] {
            assert!(
                all.iter().any(|e| e.name.contains(needle)),
                "introspection must cover `{needle}`; got names like {:?}",
                all.iter().map(|e| &e.name).take(10).collect::<Vec<_>>()
            );
        }
        assert!(!filter(&all, "prompt").is_empty());
        assert!(filter(&all, "zzz-no-such-thing").is_empty());
    }
}
