use clap::ArgMatches;

use archetect_core::Archetect;
use archetect_core::catalog::catalog_index::{IndexEntry, IndexEntryKind};
use archetect_core::catalog::catalog_indexer::CatalogIndexer;

/// Print the resolved catalog tree. Recursively resolves remote
/// sub-catalogs from the cache (or fetches if not offline), so the
/// output matches what the user sees when browsing interactively.
///
/// An optional path argument drills into a subtree:
///   archetect ls              → full tree
///   archetect ls rust         → entries under "rust"
///   archetect ls rust/cli     → entries under "rust/cli"
pub fn handle_commands_subcommand(args: &ArgMatches, archetect: &Archetect) {
    let catalog = match archetect.configuration().catalog() {
        Some(c) if !c.is_empty() => c,
        _ => {
            println!("(no catalog entries available)");
            return;
        }
    };

    let index = CatalogIndexer::new(archetect.clone()).build_index(catalog);

    let path = args
        .get_one::<String>("ls-path")
        .map(String::as_str)
        .unwrap_or("");

    match index.browse(path) {
        Some(entries) if !entries.is_empty() => {
            print_entries(entries, 0);
        }
        Some(_) => {
            if path.is_empty() {
                println!("(no catalog entries available)");
            } else {
                println!("(no entries under '{}')", path);
            }
        }
        None => {
            println!("(path '{}' not found in catalog)", path);
        }
    }
}

fn print_entries(entries: &[IndexEntry], depth: usize) {
    let indent = "  ".repeat(depth);
    for entry in entries {
        if !entry.children.is_empty() || entry.kind == IndexEntryKind::Group {
            let icon = "📂";
            if entry.description != entry.name {
                println!("{}  {} {} — {}", indent, icon, entry.name, entry.description);
            } else {
                println!("{}  {} {}", indent, icon, entry.name);
            }
            print_entries(&entry.children, depth + 1);
        } else {
            let icon = "📦";
            if entry.description != entry.name {
                println!("{}  {} {} — {}", indent, icon, entry.name, entry.description);
            } else {
                println!("{}  {} {}", indent, icon, entry.name);
            }
        }
    }
}
