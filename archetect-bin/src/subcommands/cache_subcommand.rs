use std::fs;

use clap::ArgMatches;
use inquire::Confirm;
use log::{error, info};

use archetect_core::Archetect;
use archetect_core::catalog::PreCacher;
use archetect_core::errors::ArchetectError;

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

/// Pull a source itself (a catalog's own repository too) and everything reachable from it.
fn handle_pull(source: &str, archetect: &Archetect) -> Result<(), ArchetectError> {
    info!("Pulling {}", source);
    let stats = PreCacher::new(archetect.clone()).pull_source(source)?;
    info!(
        "Pre-cache complete: {} pulled, {} skipped, {} failed, {} child manifests walked",
        stats.pulled, stats.skipped, stats.failed, stats.manifests_walked
    );
    Ok(())
}

/// Invalidate a source itself (a catalog's own repository too) and everything reachable from it.
fn handle_invalidate(source: &str, archetect: &Archetect) -> Result<(), ArchetectError> {
    info!("Invalidating {}", source);
    let stats = PreCacher::new(archetect.clone()).invalidate_source(source)?;
    info!(
        "Invalidation complete: {} invalidated, {} skipped, {} failed",
        stats.pulled, stats.skipped, stats.failed
    );
    Ok(())
}

/// Wipe the entire cache directory. Asks first on a terminal; with no terminal to ask on it
/// refuses out loud (pass `--headless` to clear without asking) rather than doing nothing silently.
fn handle_clear(archetect: &Archetect) -> Result<(), ArchetectError> {
    let cache_dir = archetect.layout().cache_dir();
    if !archetect.is_headless() {
        let prompt = Confirm::new("Are you sure you want to remove all cached Archetypes and Catalogs?")
            .with_default(false);
        match prompt.prompt() {
            Ok(true) => {}
            Ok(false) => {
                info!("Cache left as it is: {cache_dir}");
                return Ok(());
            }
            Err(err) => {
                return Err(ArchetectError::ConfigError(format!(
                    "cache clear asks before removing {cache_dir}, and could not ask ({err}); \
                     run it with --headless to clear without asking"
                )));
            }
        }
    }

    let mut removed = 0usize;
    if cache_dir.exists() {
        for path in fs::read_dir(&cache_dir)?.flatten() {
            let path = path.path();
            if path.is_dir() {
                fs::remove_dir_all(&path)?;
            } else {
                fs::remove_file(&path)?;
            }
            removed += 1;
        }
    }
    info!("Cache cleared: {cache_dir} ({removed} entries removed)");
    Ok(())
}
