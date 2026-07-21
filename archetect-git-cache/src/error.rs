/// Errors from the git source cache. Kept host-agnostic — variants map cleanly onto archetect's
/// `SourceError` (via a `From` on that side) and onto prova's `String` errors.
#[derive(Debug, thiserror::Error)]
pub enum GitCacheError {
    /// The cache dir doesn't exist and offline mode forbids cloning it.
    #[error("Remote source is not cached and offline mode is set: `{0}`")]
    OfflineAndNotCached(String),
    /// The requested ref isn't in the cache and offline mode forbids fetching it.
    #[error("Ref `{0}` not found in cache and running offline")]
    RefNotCachedOffline(String),
    /// A source with no explicit ref had no discoverable `main`/`master` default branch.
    #[error("Failed to find a default `main`/`master` branch")]
    NoDefaultBranch,
    /// A remote operation (clone/fetch/ls-remote) failed, including the `git` CLI fallback.
    #[error("Remote source error: `{0}`")]
    Remote(String),
    /// A local libgit2 operation failed.
    #[error("Git error: `{0}`")]
    Git(#[from] git2::Error),
    /// A filesystem operation failed.
    #[error("IO error: `{0}`")]
    Io(#[from] std::io::Error),
}
