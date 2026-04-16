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

/// Whether an entry should be rendered at all given the current opts
/// and its ancestry. An entry is hidden when:
///   - `show: false` in the yaml (opt-out), or
///   - it lives under an archetype ancestor (it's a component there)
/// — unless `-a` was passed.
fn should_display(entry: &IndexEntry, under_archetype: bool, opts: &DisplayOpts) -> bool {
    if opts.show_all {
        return true;
    }
    if !entry.show {
        return false;
    }
    if under_archetype {
        return false;
    }
    true
}

fn print_entries(entries: &[IndexEntry], depth: usize, under_archetype: bool, opts: &DisplayOpts) {
    for entry in entries {
        if !should_display(entry, under_archetype, opts) {
            continue;
        }
        print_one(entry, depth);
        let next_under = under_archetype || entry.is_archetype;
        print_entries(&entry.children, depth + 1, next_under, opts);
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
    under_archetype: bool,
    opts: &DisplayOpts,
    any: &mut bool,
) {
    let target_prefix = format!("{}/", filter);
    for entry in entries {
        let is_target = entry.path == filter;
        let is_descendant = entry.path.starts_with(&target_prefix);
        let is_ancestor = filter.starts_with(&format!("{}/", entry.path));

        if is_target || is_descendant {
            // When the user explicitly drills into a path, respect their
            // intent: `-a` still controls components / hidden entries,
            // but the target itself is always shown.
            if is_target || should_display(entry, under_archetype, opts) {
                *any = true;
                print_one(entry, depth);
                let next_under = under_archetype || entry.is_archetype;
                print_entries(&entry.children, depth + 1, next_under, opts);
            }
        } else if is_ancestor {
            *any = true;
            print_one(entry, depth);
            let next_under = under_archetype || entry.is_archetype;
            print_filtered(&entry.children, filter, depth + 1, next_under, opts, any);
        }
    }
}

fn print_one(entry: &IndexEntry, depth: usize) {
    let indent = "  ".repeat(depth);
    let icon = icon_for(entry);
    if entry.description != entry.name {
        println!("{}  {} {} — {}", indent, icon, entry.name, entry.description);
    } else {
        println!("{}  {} {}", indent, icon, entry.name);
    }
}

fn icon_for(entry: &IndexEntry) -> &'static str {
    if !entry.show {
        // Hidden entry (shown only with -a). Mark as component-ish
        // regardless of whether it's an archetype source or a group.
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
