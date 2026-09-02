//! The eager half of the two pull modes: a background loop that keeps the server's git cache
//! fresh, so request paths (browse / describe / render) never pause for the network.
//!
//! The division of labor with the cache crate: request-path resolves run `PullPolicy::IfMissing`
//! (set by the server's `UpdateStrategy::Eager`), serving whatever tree is cached and fetching
//! only a genuinely absent source (a cold miss must still work). This loop owns freshness: every
//! `updates.refresh_interval` it walks the configured catalog with gated resolves whose TTL is
//! that same cadence — one `ls-remote` per moving ref per cycle, a fetch only for what moved.
//! A moved ref materializes a new immutable tree; in-flight renders keep leases on their old one.

use std::time::Duration;

use tracing::{info, warn};

use crate::catalog::PreCacher;
use crate::errors::ArchetectError;
use crate::Archetect;

/// Run one synchronous refresh (so a starting server binds its port only once the catalog is
/// warm), then keep refreshing in a background task for the life of the process.
///
/// Callers gate on [`UpdateStrategy::Eager`](crate::configuration::UpdateStrategy) — a lazy
/// process has no business here.
pub(crate) async fn start(prototype: Archetect) -> Result<(), ArchetectError> {
    let interval = prototype
        .configuration()
        .updates()
        .refresh_interval()
        .to_std()
        .unwrap_or_else(|_| Duration::from_secs(300));

    // First refresh, synchronously: a server that answers before its catalog is warm serves
    // cold misses with network pauses — exactly what eager mode exists to prevent.
    let warmup = prototype.clone();
    let started = std::time::Instant::now();
    tokio::task::spawn_blocking(move || refresh_once(&warmup))
        .await
        .map_err(|err| ArchetectError::ServerError(format!("catalog warm-up task failed: {err}")))?;
    info!(
        "Catalog cache warmed in {:.1}s; refreshing every {}s",
        started.elapsed().as_secs_f32(),
        interval.as_secs()
    );

    tokio::spawn(async move {
        loop {
            tokio::time::sleep(interval).await;
            let archetect = prototype.clone();
            if let Err(err) = tokio::task::spawn_blocking(move || refresh_once(&archetect)).await {
                warn!("Catalog refresh task failed: {err}");
            }
        }
    });

    Ok(())
}

/// One refresh cycle over the configured catalog. Failures are per-source and logged by the
/// walker; a source that fails one cycle is retried the next, and the cache keeps serving the
/// last good tree in the meantime.
fn refresh_once(archetect: &Archetect) {
    let Some(catalog) = archetect.configuration().catalog().cloned() else {
        return;
    };
    match PreCacher::new(archetect.clone()).refresh(&catalog) {
        Ok(stats) if stats.pulled > 0 || stats.failed > 0 => {
            info!(
                "Catalog refresh: {} refreshed, {} deduped, {} failed, {} manifests walked",
                stats.pulled, stats.skipped, stats.failed, stats.manifests_walked
            );
        }
        Ok(_) => {}
        Err(err) => warn!("Catalog refresh failed: {err}"),
    }
}
