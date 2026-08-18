use std::path::{Path, PathBuf};

use crate::ctx::GitRepoCtx;
use crate::ctx::eager_future_cell::EagerFutureCell;
use crate::ctx::git_ctx::NewGitRepoCtxError;

/// Stores information on the current active workspace.
///
/// A workspace is a directory in which `atuin` is invoked. This takes on two meanings in code due
/// to the daemon, non-daemon path.
pub struct WorkspaceCtx {
    abs_cwd: PathBuf,

    /// The git context.
    ///
    /// Git discovery is expensive (filesystem I/O), so it runs eagerly in the background from
    /// construction and is awaited on demand via [`Self::git_ctx`].
    git_ctx: EagerFutureCell<Result<Option<GitRepoCtx>, NewGitRepoCtxError>>,
}

impl WorkspaceCtx {
    /// Create a new workspace context, kicking off git discovery in the background.
    ///
    /// Panics if the current working directory cannot be determined (e.g. it was deleted out from
    /// under the process) — atuin cannot meaningfully run from a directory it cannot resolve.
    // Not `Default`: this constructor reads the cwd, spawns background work, and can panic — none
    // of which fits `Default`'s cheap-and-infallible contract.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let abs_cwd =
            std::env::current_dir().expect("failed to determine the current working directory");

        let discover_from = abs_cwd.clone();
        Self {
            git_ctx: EagerFutureCell::new(move || GitRepoCtx::new(&discover_from)),
            abs_cwd,
        }
    }

    /// Absolute path to the current working directory.
    pub fn cwd(&self) -> &Path {
        &self.abs_cwd
    }

    /// Grab a handle to the active git repo.
    ///
    /// Returns `Ok(Option::None)` if the cwd is not a git repo.
    /// Returns `Err(NewGitRepoCtxError)` if there was an error querying the git context.
    pub async fn git_ctx(&self) -> Result<Option<&GitRepoCtx>, &NewGitRepoCtxError> {
        self.git_ctx.get().await.as_ref().map(Option::as_ref)
    }
}
