use clap::ArgMatches;

use archetect_core::Archetect;
use archetect_core::catalog::catalog_index::{IndexEntry, IndexEntryKind};
use archetect_core::catalog::catalog_indexer::CatalogIndexer;

/// Print the resolved catalog tree. Recursively resolves remote
/// sub-catalogs from the cache (or fetches if not offline), so the
/// output matches what the user sees when browsing interactively.
///
/// An optional path acts as a **filter**: the tree is still rooted
/// at the top, but only the ancestors, the target, and its descendants
/// are shown. This keeps every visible indent a path you could actually
/// dispatch with `archetect <path>`.
///
///   archetect ls                    → full tree
///   archetect ls archetect          → archetect subtree with 'archetect' as root
///   archetect ls archetect/rust     → path down to 'rust' + rust's subtree
pub fn handle_commands_subcommand(args: &ArgMatches, archetect: &Archetect) {
    let catalog = match archetect.configuration().catalog() {
        Some(c) if !c.is_empty() => c,
        _ => {
            println!("(no catalog entries available)");
            return;
        }
    };

    let index = CatalogIndexer::new(archetect.clone()).build_index(catalog);

    let filter = args
        .get_one::<String>("ls-path")
        .map(String::as_str)
        .unwrap_or("")
        .trim_matches('/');

    if filter.is_empty() {
        print_entries(index.root(), 0);
        return;
    }

    let mut any = false;
    print_filtered(index.root(), filter, 0, &mut any);
    if !any {
        println!("(path '{}' not found in catalog)", filter);
    }
}

/// Print entries whose lineage includes `filter`:
/// - Ancestors of `filter` (shown solo, only the one child that leads toward the target).
/// - The target itself (plus its full subtree).
/// - Anything else is skipped.
fn print_filtered(entries: &[IndexEntry], filter: &str, depth: usize, any: &mut bool) {
    let target_prefix = format!("{}/", filter);
    for entry in entries {
        let is_target = entry.path == filter;
        let is_descendant = entry.path.starts_with(&target_prefix);
        let is_ancestor = filter.starts_with(&format!("{}/", entry.path));

        if is_target || is_descendant {
            *any = true;
            print_one(entry, depth);
            print_entries(&entry.children, depth + 1);
        } else if is_ancestor {
            *any = true;
            print_one(entry, depth);
            print_filtered(&entry.children, filter, depth + 1, any);
        }
    }
}

fn print_entries(entries: &[IndexEntry], depth: usize) {
    for entry in entries {
        print_one(entry, depth);
        print_entries(&entry.children, depth + 1);
    }
}

fn print_one(entry: &IndexEntry, depth: usize) {
    let indent = "  ".repeat(depth);
    let icon = if !entry.children.is_empty() || entry.kind == IndexEntryKind::Group {
        "📂"
    } else {
        "📦"
    };
    if entry.description != entry.name {
        println!("{}  {} {} — {}", indent, icon, entry.name, entry.description);
    } else {
        println!("{}  {} {}", indent, icon, entry.name);
    }
}
