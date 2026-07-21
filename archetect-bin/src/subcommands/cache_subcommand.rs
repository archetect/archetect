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
            let sources = resolve_sources(sub_args, archetect)?;
            for source in &sources {
                handle_pull(source, archetect)?;
            }
        }
        Some(("invalidate", sub_args)) => {
            let sources = resolve_sources(sub_args, archetect)?;
            for source in &sources {
                handle_invalidate(source, archetect)?;
            }
        }
        Some(("clear", _args)) => {
            handle_clear(archetect)?;
        }
        Some(("prune", _args)) => {
            let (removed, kept, in_use) = archetect.prune_cache()?;
            info!("Prune complete: {removed} removed, {kept} kept, {in_use} in use");
        }
        Some((command_name, _args)) => {
            error!("Unimplemented command: cache {}", command_name);
        }
    }

    Ok(())
}

/// If `source` arg is provided, use it. Otherwise, pull each top-level
/// entry from the configured catalog. Each entry is itself a catalog
/// or archetype that `handle_pull` walks recursively.
fn resolve_sources(args: &ArgMatches, archetect: &Archetect) -> Result<Vec<String>, ArchetectError> {
    if let Some(source) = args.get_one::<String>("source") {
        return Ok(vec![source.clone()]);
    }

    let catalog = archetect.configuration().catalog().ok_or_else(|| {
        ArchetectError::ConfigError(
            "No catalog configured. Provide an explicit <source> or configure a catalog.".to_string(),
        )
    })?;

    let sources: Vec<String> = catalog
        .values()
        .filter_map(|entry| entry.source.clone())
        .collect();

    if sources.is_empty() {
        return Err(ArchetectError::ConfigError(
            "Configured catalog has no source entries to pull.".to_string(),
        ));
    }

    Ok(sources)
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
    let confirmed = if archetect.is_headless() {
        // In headless / non-TTY mode, skip the interactive prompt
        true
    } else {
        let prompt = Confirm::new("Are you sure you want to remove all cached Archetypes and Catalogs?")
            .with_default(false);
        matches!(prompt.prompt(), Ok(true))
    };

    if confirmed {
        let cache_dir = archetect.layout().cache_dir();
        if cache_dir.exists() {
            let paths = fs::read_dir(cache_dir)?;
            for path in paths.flatten() {
                fs::remove_dir_all(path.path())?;
            }
            info!("Cache cleared");
        }
    }
    Ok(())
}
