use clap::ArgMatches;

use archetect_core::Archetect;
use archetect_core::catalog::catalog_index::{IndexEntry, IndexEntryKind};
use archetect_core::catalog::catalog_indexer::CatalogIndexer;

/// Print the resolved catalog tree.
///
/// Entries fall into three display categories:
///
///   📦 Archetype — resolved source has archetype.lua. Renderable.
///      May have catalog entries of its own (components it composes
///      in), but those are hidden by default.
///   📂 Catalog   — navigation node. Either inline `catalog:` entries
///      or a source whose manifest has a `catalog:` but no
///      archetype.lua.
///   🧩 Component — either declared inside an archetype's catalog, or
///      marked `show: false` in the yaml. Hidden by default, surfaced
///      by `-a` / `--all`.
///
/// An optional path acts as a filter (preserves ancestor context so
/// every visible indent is a dispatchable `archetect <path>`):
///
///   archetect ls                    → tree, archetypes + catalogs only
///   archetect ls -a                 → include components / hidden entries
///   archetect ls archetect/rust     → path + subtree (filtered)
pub fn handle_commands_subcommand(args: &ArgMatches, archetect: &Archetect) {
    let catalog = match archetect.configuration().catalog() {
        Some(c) if !c.is_empty() => c,
        _ => {
            println!("(no catalog entries available)");
            return;
        }
    };

    let index = CatalogIndexer::new(archetect.clone()).build_index(catalog);

    let show_all = args.get_flag("all");
    let filter = args
        .get_one::<String>("ls-path")
        .map(String::as_str)
        .unwrap_or("")
        .trim_matches('/');

    let opts = DisplayOpts { show_all };

    if filter.is_empty() {
        print_entries(index.root(), 0, false, &opts);
        return;
    }

    let mut any = false;
    print_filtered(index.root(), filter, 0, false, &opts, &mut any);
    if !any {
        println!("(path '{}' not found in catalog)", filter);
    }
}

struct DisplayOpts {
    show_all: bool,
}

/// An entry is classified as a **component** — and thus hidden in the
/// default view and rendered with the 🧩 icon — when ANY of:
///   - It has `show: false` in the yaml.
///   - Any ancestor has `show: false`.
///   - Any ancestor is a renderable archetype (the entry is a component
///     contributing to that archetype's composition, not a navigable
///     sibling).
/// These conditions propagate down the tree: `inside_component_scope`
/// stays true for all descendants once it flips.
fn is_component(entry: &IndexEntry, inside_component_scope: bool) -> bool {
    inside_component_scope || !entry.show
}

fn should_display(entry: &IndexEntry, inside_component_scope: bool, opts: &DisplayOpts) -> bool {
    opts.show_all || !is_component(entry, inside_component_scope)
}

fn print_entries(entries: &[IndexEntry], depth: usize, inside_component_scope: bool, opts: &DisplayOpts) {
    for entry in entries {
        let component = is_component(entry, inside_component_scope);
        if !opts.show_all && component {
            continue;
        }
        print_one(entry, depth, component);
        // Once we've entered component scope, we stay there.
        let next_scope = inside_component_scope || !entry.show || entry.is_archetype;
        print_entries(&entry.children, depth + 1, next_scope, opts);
    }
}

/// Print entries whose lineage includes `filter`:
/// - Ancestors of `filter` (shown solo on the path to the target).
/// - The target itself (plus its full subtree).
/// - Anything else skipped.
fn print_filtered(
    entries: &[IndexEntry],
    filter: &str,
    depth: usize,
    inside_component_scope: bool,
    opts: &DisplayOpts,
    any: &mut bool,
) {
    let target_prefix = format!("{}/", filter);
    for entry in entries {
        let is_target = entry.path == filter;
        let is_descendant = entry.path.starts_with(&target_prefix);
        let is_ancestor = filter.starts_with(&format!("{}/", entry.path));
        let component = is_component(entry, inside_component_scope);

        if is_target || is_descendant {
            // User explicitly drilled into a path: the target is always
            // shown. `-a` still controls descendants' component filter.
            let allow = is_target || should_display(entry, inside_component_scope, opts);
            if allow {
                *any = true;
                print_one(entry, depth, component);
                let next_scope = inside_component_scope || !entry.show || entry.is_archetype;
                print_entries(&entry.children, depth + 1, next_scope, opts);
            }
        } else if is_ancestor {
            *any = true;
            print_one(entry, depth, component);
            let next_scope = inside_component_scope || !entry.show || entry.is_archetype;
            print_filtered(&entry.children, filter, depth + 1, next_scope, opts, any);
        }
    }
}

fn print_one(entry: &IndexEntry, depth: usize, is_component: bool) {
    let indent = "  ".repeat(depth);
    let icon = icon_for(entry, is_component);
    if entry.description != entry.name {
        println!("{}  {} {} — {}", indent, icon, entry.name, entry.description);
    } else {
        println!("{}  {} {}", indent, icon, entry.name);
    }
}

fn icon_for(entry: &IndexEntry, is_component: bool) -> &'static str {
    // Federation root: the entry itself carries a `server:` (its local
    // path equals its own remote-info prefix with an empty remote_path).
    // Flag it with a satellite so users can tell local trees from
    // remote-served ones at a glance. Descendants of remote entries
    // keep their normal archetype/group icons — they're "under" the
    // satellite's domain visually from the indent alone.
    if let Some(remote) = &entry.remote {
        if remote.local_prefix == entry.path {
            return "🛰️ ";
        }
    }
    if is_component {
        // Component (hidden-by-default) — shown only with -a. Flag it
        // regardless of whether the entry itself is an archetype
        // source or a group of other components.
        return "🧩";
    }
    if entry.is_archetype {
        "📦"
    } else if entry.kind == IndexEntryKind::Group {
        "📂"
    } else {
        // Unresolved leaf — treat as archetype for display (best guess,
        // since we couldn't inspect the source).
        "📦"
    }
}
