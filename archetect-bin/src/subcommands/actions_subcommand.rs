use clap::ArgMatches;

use archetect_core::Archetect;
use archetect_core::manifest::CatalogEntry;
use linked_hash_map::LinkedHashMap;

/// Print the resolved catalog tree (project's catalog if a `.archetect.yaml` is
/// present in CWD, otherwise the global catalog).
///
/// Output:
///   📦 services/grpc — gRPC Service
///   📦 services/rest — REST Service
///   📦 libraries — Shared Libraries
pub fn handle_commands_subcommand(_args: &ArgMatches, archetect: &Archetect) {
    match archetect.configuration().catalog() {
        Some(catalog) if !catalog.is_empty() => {
            print_entries(catalog, "");
        }
        _ => {
            println!("(no catalog entries available)");
        }
    }
}

fn print_entries(entries: &LinkedHashMap<String, CatalogEntry>, prefix: &str) {
    for (name, entry) in entries {
        let path = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{}/{}", prefix, name)
        };

        let icon = if entry.is_group() { "📂" } else { "📦" };
        let label = entry.display_description(name);
        if label != *name {
            println!("  {} {} — {}", icon, path, label);
        } else {
            println!("  {} {}", icon, path);
        }

        if let Some(ref nested) = entry.catalog {
            print_entries(nested, &path);
        }
    }
}
