use std::fs;

use clap::ArgMatches;
use inquire::Confirm;
use log::{error, info};

use archetect_core::Archetect;
use archetect_core::catalog::PreCacher;
use archetect_core::errors::ArchetectError;
use archetect_core::manifest::Manifest;

pub fn handle_cache_subcommand(args: &ArgMatches, archetect: &Archetect) -> Result<(), ArchetectError> {
    match args.subcommand() {
        None => {
            error!("Subcommand expected");
        }
        Some(("pull", sub_args)) => {
            let source = sub_args.get_one::<String>("source").expect("Enforced by Clap");
            handle_pull(source, archetect)?;
        }
        Some(("invalidate", sub_args)) => {
            let source = sub_args.get_one::<String>("source").expect("Enforced by Clap");
            handle_invalidate(source, archetect)?;
        }
        Some(("clear", _args)) => {
            handle_clear(archetect)?;
        }
        Some((command_name, _args)) => {
            error!("Unimplemented command: cache {}", command_name);
        }
    }

    Ok(())
}

/// Recursively pull a source and everything reachable from its catalog tree.
fn handle_pull(source: &str, archetect: &Archetect) -> Result<(), ArchetectError> {
    info!("Pulling {}", source);

    // Resolve the root source — this triggers an initial clone/fetch if needed
    let resolved = archetect.new_source(source)?;
    let root_path = resolved.path()?;

    // Load the root manifest and walk its catalog tree
    let manifest = Manifest::load(root_path)?;
    let stats = PreCacher::new(archetect.clone()).pull(&manifest)?;

    info!(
        "Pre-cache complete: {} pulled, {} skipped, {} failed, {} child manifests walked",
        stats.pulled, stats.skipped, stats.failed, stats.manifests_walked
    );
    Ok(())
}

/// Recursively invalidate the cache for a source and everything reachable.
fn handle_invalidate(source: &str, archetect: &Archetect) -> Result<(), ArchetectError> {
    info!("Invalidating {}", source);

    let resolved = archetect.new_source(source)?;
    let root_path = resolved.path()?;

    let manifest = Manifest::load(root_path)?;
    let stats = PreCacher::new(archetect.clone()).invalidate(&manifest)?;

    info!(
        "Invalidation complete: {} invalidated, {} skipped, {} failed",
        stats.pulled, stats.skipped, stats.failed
    );
    Ok(())
}

/// Wipe the entire cache directory.
fn handle_clear(archetect: &Archetect) -> Result<(), ArchetectError> {
    let prompt = Confirm::new("Are you sure you want to remove all cached Archetypes and Catalogs?")
        .with_default(false);
    if let Ok(true) = prompt.prompt() {
        let paths = fs::read_dir(archetect.layout().cache_dir())?;
        for path in paths.flatten() {
            fs::remove_dir_all(path.path())?;
        }
    }
    Ok(())
}
