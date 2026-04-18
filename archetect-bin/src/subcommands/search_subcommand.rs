use clap::ArgMatches;

use archetect_core::Archetect;
use archetect_core::catalog::catalog_index::{IndexEntry, IndexEntryKind};
use archetect_core::catalog::catalog_indexer::CatalogIndexer;

/// Full-text search across the resolved catalog. Mirrors the MCP
/// `catalog_search` tool — matches name, description, path, and
/// metadata fields (languages, frameworks, tags). All terms must
/// match (AND semantics).
///
/// Example output:
///
///   archetect search rust cli
///   📦 archetect/rust/cli — Rust CLI application (clap derive, xtask workflow)
///   📦 common/starters/archetype-starter — Scaffold a new Archetect archetype
pub fn handle_search_subcommand(args: &ArgMatches, archetect: &Archetect) {
    // Collect from both `terms` (this subcommand's positional, variadic)
    // and `action` (the top-level global positional). Without this, the
    // single-arg form `archetect search foo` would route "foo" into the
    // global `action` slot and `terms` would come back empty.
    let mut terms: Vec<String> = args
        .get_many::<String>("terms")
        .map(|values| values.cloned().collect())
        .unwrap_or_default();
    if let Some(action) = args.get_one::<String>("action") {
        if action != "default" && !terms.contains(action) {
            terms.insert(0, action.clone());
        }
    }

    if terms.is_empty() {
        println!("(no search terms supplied)");
        return;
    }

    let catalog = match archetect.configuration().catalog() {
        Some(c) if !c.is_empty() => c,
        _ => {
            println!("(no catalog entries available)");
            return;
        }
    };

    let index = CatalogIndexer::new(archetect.clone()).build_index(catalog);
    let query = terms.join(" ");
    let show_all = args.get_flag("all");

    let results: Vec<&IndexEntry> = index
        .search(&query)
        .into_iter()
        .filter(|entry| show_all || entry.show)
        .collect();

    if results.is_empty() {
        println!("(no matches for '{}')", query);
        return;
    }

    for entry in &results {
        print_result(entry);
    }
    println!();
    println!("{} match(es) for '{}'", results.len(), query);
}

fn print_result(entry: &IndexEntry) {
    let icon = icon_for(entry);
    if entry.description != entry.name {
        println!("  {} {} — {}", icon, entry.path, entry.description);
    } else {
        println!("  {} {}", icon, entry.path);
    }
}

fn icon_for(entry: &IndexEntry) -> &'static str {
    // Federation root: flag with satellite. See actions_subcommand.rs
    // for the reasoning.
    if let Some(remote) = &entry.remote {
        if remote.local_prefix == entry.path {
            return "🛰️ ";
        }
    }
    if !entry.show {
        return "🧩";
    }
    if entry.is_archetype {
        "📦"
    } else if entry.kind == IndexEntryKind::Group {
        "📂"
    } else {
        "📦"
    }
}
