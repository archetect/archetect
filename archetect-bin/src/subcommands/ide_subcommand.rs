use std::fs;
use std::path::{Path, PathBuf};

use archetect_core::errors::ArchetectError;
use archetect_core::manifest::MANIFEST_FILE_NAMES;
use archetect_core::system::SystemLayout;
use serde_json::{json, Value};

/// The annotation stubs ship embedded in archetect-core (`help::CORE_STUBS`) — one embedding,
/// two sinks: runtime introspection (`archetect introspect`) and this IDE install.
use archetect_core::help::CORE_STUBS;

/// How `ide setup` treats the project's `.luarc.json` pointer. The annotation stubs are always
/// (re)installed; this governs only whether — and how — the `.luarc.json` at the project root is
/// written. It mirrors prova's policy so a project wired for both tools behaves identically under
/// each: `<tool> ide setup` merges non-destructively, and neither ever clobbers the other's entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Manage {
    /// Create `.luarc.json` if absent; if one exists that we wrote, refresh it; if it's a file the
    /// user owns, leave it and print a hint. The polite default for automatic contexts.
    Auto,
    /// Always create-or-merge our entry into `.luarc.json`, even one the user owns. The explicit
    /// opt-in `ide setup` uses by default — the user asked us to wire it up.
    Always,
    /// Never touch `.luarc.json`; only (re)install the annotation stubs.
    Never,
}

impl Manage {
    /// Parse a `--manage` value, defaulting to `Always` (the `ide setup` default — an explicit ask).
    pub fn parse(value: Option<&str>) -> Result<Manage, ArchetectError> {
        match value {
            None | Some("always") => Ok(Manage::Always),
            Some("auto") => Ok(Manage::Auto),
            Some("never") => Ok(Manage::Never),
            Some(other) => Err(ArchetectError::GeneralError(format!(
                "invalid --manage {other:?} (expected \"auto\", \"always\", or \"never\")"
            ))),
        }
    }
}

pub fn handle_ide_subcommand(layout: &dyn SystemLayout, manage: Manage) -> Result<(), ArchetectError> {
    let annotations_dir = install_annotations(layout)?;
    maybe_manage_luarc(&annotations_dir, manage)?;
    Ok(())
}

fn install_annotations(layout: &dyn SystemLayout) -> Result<PathBuf, ArchetectError> {
    let annotations_dir = PathBuf::from(layout.data_dir().join("lua/annotations").as_str());

    fs::create_dir_all(&annotations_dir)
        .map_err(|e| ArchetectError::GeneralError(format!("Failed to create {}: {}", annotations_dir.display(), e)))?;

    for (name, contents) in CORE_STUBS {
        let path = annotations_dir.join(name);
        fs::write(&path, contents)
            .map_err(|e| ArchetectError::GeneralError(format!("Failed to write {}: {}", path.display(), e)))?;
    }

    eprintln!("archetect: Lua annotations installed to {}", annotations_dir.display());
    Ok(annotations_dir)
}

/// Reconcile the project's `.luarc.json` with our annotations entry, per `manage`. Only runs inside a
/// Lua archetype directory (a manifest **and** an `archetype.lua`); elsewhere there's nothing to wire.
///
/// The reconcile is **non-destructive**: it adds our entry if missing, sweeps only entries we
/// ourselves manage (paths under our annotations root that are no longer current — e.g. a moved XDG
/// dir), and leaves every other key and `workspace.library` entry — including another tool's, like
/// prova's — exactly as it found it. That is what lets `archetect ide setup` and `prova ide setup`
/// run in either order and both survive.
fn maybe_manage_luarc(annotations_dir: &Path, manage: Manage) -> Result<(), ArchetectError> {
    let cwd = std::env::current_dir()
        .map_err(|e| ArchetectError::GeneralError(format!("Failed to get current directory: {}", e)))?;

    let has_manifest = MANIFEST_FILE_NAMES.iter().any(|name| cwd.join(name).exists());
    let has_lua_script = cwd.join("archetype.lua").exists();

    if !has_manifest {
        return Ok(());
    }
    if !has_lua_script {
        eprintln!("archetect: Archetype detected but no archetype.lua found — skipping .luarc.json");
        return Ok(());
    }
    if manage == Manage::Never {
        return Ok(());
    }

    let luarc_path = cwd.join(".luarc.json");
    let entry = path_entry(annotations_dir);
    let annotations_root = entry.clone();
    let existing = fs::read_to_string(&luarc_path).ok();

    let (new_content, action) = match (existing, manage) {
        // No file yet — create one we own (any policy but Never, handled above).
        (None, _) => (Some(fresh_luarc(&entry)?), "Created"),
        // A file we wrote can be refreshed even under Auto; a foreign file is left with a hint.
        (Some(text), Manage::Auto) => {
            if luarc_is_ours(&text) {
                (Some(merge_luarc(&text, &entry, &annotations_root, &luarc_path)?), "Updated")
            } else if luarc_has_entry(&text, &entry) {
                (None, "")
            } else {
                eprintln!(
                    "archetect: .luarc.json exists and is yours — run `archetect ide setup --manage always` to add archetect's annotations"
                );
                (None, "")
            }
        }
        // Explicit opt-in: merge into whatever is there. Stay quiet + idempotent when already wired.
        (Some(text), Manage::Always) => {
            if luarc_has_entry(&text, &entry) && luarc_runtime_set(&text) {
                (None, "")
            } else {
                (Some(merge_luarc(&text, &entry, &annotations_root, &luarc_path)?), "Updated")
            }
        }
        (Some(_), Manage::Never) => unreachable!("Never handled above"),
    };

    if let Some(content) = new_content {
        fs::write(&luarc_path, content)
            .map_err(|e| ArchetectError::GeneralError(format!("Failed to write .luarc.json: {}", e)))?;
        eprintln!("archetect: {} .luarc.json for IDE support", action);
    }

    Ok(())
}

/// A `workspace.library` entry for a path, forward-slashed so the JSON reads the same on every
/// platform (LuaLS normalizes separators itself).
fn path_entry(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

/// The keys a `.luarc.json` we created carries — used to recognize our own handiwork.
const OUR_KEYS: [&str; 3] = ["runtime.version", "workspace.library", "workspace.checkThirdParty"];

/// A fresh `.luarc.json` for a project we own the config of. `checkThirdParty: false` silences the
/// LuaLS "configure as a library?" prompt — the same shape prova writes, so a fresh file from either
/// tool is identical.
fn fresh_luarc(entry: &str) -> Result<String, ArchetectError> {
    let doc = json!({
        "runtime.version": "Lua 5.4",
        "workspace.library": [entry],
        "workspace.checkThirdParty": false,
    });
    serialize(&doc)
}

/// Did we write this `.luarc.json`? True only when it carries exactly our keys and nothing else — so
/// adding any setting of your own transfers ownership and we merge (never rewrite) from then on.
fn luarc_is_ours(text: &str) -> bool {
    match serde_json::from_str::<Value>(text) {
        Ok(Value::Object(map)) => map.len() == OUR_KEYS.len() && OUR_KEYS.iter().all(|k| map.contains_key(*k)),
        _ => false,
    }
}

/// Does `workspace.library` already list `entry`?
fn luarc_has_entry(text: &str, entry: &str) -> bool {
    match serde_json::from_str::<Value>(text) {
        Ok(Value::Object(map)) => match map.get("workspace.library") {
            Some(Value::Array(items)) => items.iter().any(|v| v.as_str() == Some(entry)),
            _ => false,
        },
        _ => false,
    }
}

/// Is `runtime.version` already set (so a merge would leave it untouched)?
fn luarc_runtime_set(text: &str) -> bool {
    matches!(
        serde_json::from_str::<Value>(text),
        Ok(Value::Object(ref map)) if map.contains_key("runtime.version")
    )
}

/// Reconcile our entry into an existing `.luarc.json`: add it if missing, drop our own stale entries
/// (paths under our annotations root that aren't the current one), set `runtime.version` only if
/// unset, and leave every other key and entry untouched. Errors (rather than corrupts) on non-JSON.
fn merge_luarc(text: &str, entry: &str, annotations_root: &str, path: &Path) -> Result<String, ArchetectError> {
    let mut doc: Value = serde_json::from_str(text).map_err(|e| {
        ArchetectError::GeneralError(format!(
            "{} is not plain JSON ({e}); add {entry:?} to workspace.library by hand, or use --manage never",
            path.display()
        ))
    })?;
    let map = doc
        .as_object_mut()
        .ok_or_else(|| ArchetectError::GeneralError(format!("{} is not a JSON object", path.display())))?;

    match map.entry("workspace.library").or_insert_with(|| json!([])) {
        Value::Array(items) => {
            // Sweep our own stale entries first (a moved annotations dir), never anyone else's.
            items.retain(|v| match v.as_str() {
                Some(s) => !is_managed(s, annotations_root) || s == entry,
                None => true,
            });
            if !items.iter().any(|v| v.as_str() == Some(entry)) {
                items.push(json!(entry));
            }
        }
        other => *other = json!([entry]),
    }
    map.entry("runtime.version").or_insert_with(|| json!("Lua 5.4"));

    serialize(&Value::Object(map.clone()))
}

/// Is this a `workspace.library` entry we manage (under our annotations root)?
fn is_managed(entry: &str, annotations_root: &str) -> bool {
    entry.starts_with(annotations_root)
}

fn serialize(doc: &Value) -> Result<String, ArchetectError> {
    let mut s = serde_json::to_string_pretty(doc)
        .map_err(|e| ArchetectError::GeneralError(format!("Failed to serialize .luarc.json: {}", e)))?;
    s.push('\n');
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ARCH: &str = "/home/me/.local/share/archetect/lua/annotations";
    const PROVA: &str = "/home/me/.local/share/prova/lua/annotations";

    fn lib(text: &str) -> Vec<String> {
        let doc: Value = serde_json::from_str(text).unwrap();
        doc["workspace.library"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect()
    }

    #[test]
    fn fresh_file_carries_our_three_keys_and_check_third_party() {
        let text = fresh_luarc(ARCH).unwrap();
        assert!(luarc_is_ours(&text));
        let doc: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(doc["workspace.checkThirdParty"], json!(false));
        assert_eq!(lib(&text), vec![ARCH.to_string()]);
    }

    /// The bug this whole change fixes: merging must never drop a foreign entry (prova's).
    #[test]
    fn merge_preserves_a_foreign_entry() {
        let existing = format!(
            "{{\n  \"runtime.version\": \"Lua 5.3\",\n  \"diagnostics.globals\": [\"vim\"],\n  \"workspace.library\": [\"{PROVA}\"]\n}}"
        );
        let merged = merge_luarc(&existing, ARCH, ARCH, Path::new(".luarc.json")).unwrap();
        let entries = lib(&merged);
        assert!(entries.contains(&PROVA.to_string()), "foreign entry was dropped: {entries:?}");
        assert!(entries.contains(&ARCH.to_string()), "our entry was not added: {entries:?}");
        let doc: Value = serde_json::from_str(&merged).unwrap();
        assert_eq!(doc["runtime.version"], "Lua 5.3", "user's runtime.version was overridden");
        assert_eq!(doc["diagnostics.globals"][0], "vim", "a foreign key was lost");
    }

    #[test]
    fn merge_is_idempotent() {
        let once = fresh_luarc(ARCH).unwrap();
        let twice = merge_luarc(&once, ARCH, ARCH, Path::new(".luarc.json")).unwrap();
        assert_eq!(lib(&twice), vec![ARCH.to_string()], "a re-merge duplicated our entry");
    }

    #[test]
    fn merge_sweeps_our_stale_entry_but_not_foreign() {
        let stale = format!("{ARCH}-OLD-XDG");
        let existing = format!("{{\n  \"workspace.library\": [\"{PROVA}\", \"{stale}\"]\n}}");
        let merged = merge_luarc(&existing, ARCH, ARCH, Path::new(".luarc.json")).unwrap();
        let entries = lib(&merged);
        assert!(!entries.contains(&stale), "our stale entry survived: {entries:?}");
        assert!(entries.contains(&PROVA.to_string()), "foreign entry was swept: {entries:?}");
        assert!(entries.contains(&ARCH.to_string()));
    }

    #[test]
    fn a_user_owned_file_is_not_recognized_as_ours() {
        let text = format!(
            "{{\n  \"runtime.version\": \"Lua 5.4\",\n  \"workspace.library\": [\"{ARCH}\"],\n  \"workspace.checkThirdParty\": false,\n  \"diagnostics.globals\": [\"vim\"]\n}}"
        );
        assert!(!luarc_is_ours(&text), "an extra key must transfer ownership");
    }

    #[test]
    fn non_json_is_an_error_not_a_clobber() {
        let err = merge_luarc("-- not json", ARCH, ARCH, Path::new(".luarc.json")).unwrap_err();
        assert!(format!("{err}").contains("not plain JSON"));
    }

    #[test]
    fn manage_parses_and_defaults_to_always() {
        assert_eq!(Manage::parse(None).unwrap(), Manage::Always);
        assert_eq!(Manage::parse(Some("auto")).unwrap(), Manage::Auto);
        assert_eq!(Manage::parse(Some("never")).unwrap(), Manage::Never);
        assert!(Manage::parse(Some("bogus")).is_err());
    }
}
