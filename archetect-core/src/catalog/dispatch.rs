//! Shared catalog dispatch logic.
//!
//! This module is the single source of truth for resolving and rendering
//! catalog entries. It is used by:
//!
//! - The archetype auto-present path (when an archetype has a `catalog` and
//!   no script — see `auto_present.rs`)
//! - The Lua `catalog.render()` function (see `script/lua/modules.rs`)
//! - The CLI dispatch (`archetect [path]` — see `archetect-bin/src/main.rs`)
//!
//! All three callers walk the same structure: a `LinkedHashMap<String, CatalogEntry>`.
//! Whether the catalog comes from a manifest or a configuration is irrelevant
//! at this layer.

use std::fmt::{Display, Formatter};

use inquire::{InquireError, Select};
use linked_hash_map::LinkedHashMap;

use crate::Archetect;
use crate::archetype::render_context::RenderContext;
use crate::errors::ArchetectError;
use crate::manifest::CatalogEntry;

/// Resolve a slash-separated path to an entry within a catalog.
///
/// Returns `None` if any segment cannot be resolved or if the path is empty.
/// Empty segments (from `//` or trailing `/`) are skipped.
pub fn resolve_path<'a>(
    catalog: &'a LinkedHashMap<String, CatalogEntry>,
    path: &str,
) -> Option<&'a CatalogEntry> {
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if segments.is_empty() {
        return None;
    }

    let mut current = catalog;
    for (i, segment) in segments.iter().enumerate() {
        let entry = current.get(*segment)?;
        if i == segments.len() - 1 {
            return Some(entry);
        }
        current = entry.catalog.as_ref()?;
    }

    None
}

/// Top-level dispatch for a catalog given an optional path.
///
/// - `path == None` → present the catalog as a menu
/// - `path` resolves to a **group** → present that group as a submenu
/// - `path` resolves to a **leaf** → render the referenced archetype
/// - `path` doesn't resolve → return an error listing available entries
pub fn dispatch(
    archetect: &Archetect,
    catalog: &LinkedHashMap<String, CatalogEntry>,
    path: Option<&str>,
    render_context: RenderContext,
) -> Result<(), ArchetectError> {
    match path {
        None | Some("") => present_entries(archetect, catalog, &render_context),
        Some(p) => {
            let entry = resolve_path(catalog, p).ok_or_else(|| {
                ArchetectError::GeneralError(format!(
                    "Catalog path '{}' not found. Available top-level entries: {:?}",
                    p,
                    catalog.keys().collect::<Vec<_>>()
                ))
            })?;

            if entry.is_group() {
                let nested = entry.catalog.as_ref().ok_or_else(|| {
                    ArchetectError::GeneralError(format!(
                        "Catalog entry '{}' is marked as a group but has no children",
                        p
                    ))
                })?;
                present_entries(archetect, nested, &render_context)
            } else {
                render_leaf(archetect, entry, p, render_context)
            }
        }
    }
}

/// Render a leaf catalog entry — i.e., resolve its source and render the archetype.
/// Applies any pre-configured answers, switches, and defaults from the entry.
pub fn render_leaf(
    archetect: &Archetect,
    entry: &CatalogEntry,
    path: &str,
    mut render_context: RenderContext,
) -> Result<(), ArchetectError> {
    let source = entry.source.as_ref().ok_or_else(|| {
        ArchetectError::GeneralError(format!(
            "Catalog entry '{}' has no source", path
        ))
    })?;

    // Apply pre-configured answers from the catalog entry
    if let Some(ref answers) = entry.answers {
        for (k, v) in answers {
            render_context.answers_mut().insert(k.clone(), v.clone());
        }
    }
    if let Some(ref switches) = entry.switches {
        render_context.set_switches(switches.clone());
    }
    if let Some(ref use_defaults) = entry.use_defaults {
        render_context.set_use_defaults(use_defaults.clone());
    }
    if let Some(true) = entry.use_defaults_all {
        render_context.set_use_defaults_all(true);
    }

    let child = archetect.new_archetype(source)?;
    child.check_requirements()?;
    child.render(render_context)?;
    Ok(())
}

/// Present catalog entries interactively as a select menu.
/// Groups recurse, leaves render the archetype, then loop back.
pub fn present_entries(
    archetect: &Archetect,
    entries: &LinkedHashMap<String, CatalogEntry>,
    render_context: &RenderContext,
) -> Result<(), ArchetectError> {
    if entries.is_empty() {
        return Ok(());
    }

    loop {
        let choices: Vec<EntryItem> = entries
            .iter()
            .enumerate()
            .map(|(idx, (name, entry))| {
                let icon = if entry.is_group() { "📂" } else { "📦" };
                let label = entry.display_description(name);
                let width = if entries.len() <= 99 { 2 } else { 3 };
                EntryItem {
                    text: format!("{:>0width$}: {} {}", idx + 1, icon, label),
                    name: name.clone(),
                    entry: entry.clone(),
                }
            })
            .collect();

        let prompt = Select::new("Select:", choices).with_page_size(30);

        match prompt.prompt() {
            Ok(item) => {
                if item.entry.is_group() {
                    if let Some(ref nested) = item.entry.catalog {
                        present_entries(archetect, nested, render_context)?;
                    }
                } else {
                    render_leaf(archetect, &item.entry, &item.name, render_context.clone())?;
                }
            }
            Err(InquireError::OperationCanceled | InquireError::OperationInterrupted) => {
                return Ok(());
            }
            Err(err) => {
                return Err(ArchetectError::GeneralError(err.to_string()));
            }
        }
    }
}

struct EntryItem {
    text: String,
    name: String,
    #[allow(dead_code)]
    entry: CatalogEntry,
}

impl Display for EntryItem {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;
    use crate::manifest::Manifest;

    fn test_catalog() -> LinkedHashMap<String, CatalogEntry> {
        let yaml = indoc! {r#"
            description: "Test"
            requires:
              archetect: "3.0.0"
            catalog:
              services:
                description: "Backend Services"
                catalog:
                  grpc:
                    description: "gRPC Service"
                    source: "git@github.com:org/grpc.git"
                  rest:
                    description: "REST Service"
                    source: "git@github.com:org/rest.git"
              libraries:
                description: "Shared Libraries"
                source: "git@github.com:org/libs.git"
        "#};
        let manifest: Manifest = serde_yaml::from_str(yaml).unwrap();
        manifest.catalog.unwrap()
    }

    #[test]
    fn test_resolve_top_level_leaf() {
        let catalog = test_catalog();
        let entry = resolve_path(&catalog, "libraries").unwrap();
        assert!(entry.is_leaf());
        assert_eq!(entry.source.as_deref(), Some("git@github.com:org/libs.git"));
    }

    #[test]
    fn test_resolve_top_level_group() {
        let catalog = test_catalog();
        let entry = resolve_path(&catalog, "services").unwrap();
        assert!(entry.is_group());
    }

    #[test]
    fn test_resolve_nested_leaf() {
        let catalog = test_catalog();
        let entry = resolve_path(&catalog, "services/grpc").unwrap();
        assert!(entry.is_leaf());
        assert_eq!(entry.source.as_deref(), Some("git@github.com:org/grpc.git"));
    }

    #[test]
    fn test_resolve_missing_returns_none() {
        let catalog = test_catalog();
        assert!(resolve_path(&catalog, "nonexistent").is_none());
        assert!(resolve_path(&catalog, "services/nonexistent").is_none());
    }

    #[test]
    fn test_resolve_empty_returns_none() {
        let catalog = test_catalog();
        assert!(resolve_path(&catalog, "").is_none());
        assert!(resolve_path(&catalog, "/").is_none());
    }

    #[test]
    fn test_resolve_path_into_leaf_returns_none() {
        // Asking for a sub-path under a leaf should fail (leaves have no children)
        let catalog = test_catalog();
        assert!(resolve_path(&catalog, "libraries/something").is_none());
    }

    #[test]
    fn test_resolve_strips_leading_slash() {
        let catalog = test_catalog();
        let entry = resolve_path(&catalog, "/services/grpc").unwrap();
        assert_eq!(entry.source.as_deref(), Some("git@github.com:org/grpc.git"));
    }
}
