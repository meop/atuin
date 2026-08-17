use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;

use crate::ctx::GitRepoCtx;
use crate::ctx::git_ctx::NewGitRepoCtxError;

#[derive(Debug, Error)]
pub enum NewWorkspaceError {
    #[error("failed to discover the active working directory: {0}")]
    GetCwdError(io::Error),
}

/// Stores information on the current active workspace.
///
/// A workspace is a directory in which `atuin` is invoked. This takes on two meanings in code due
/// to the daemon, non-daemon path.
#[derive(Debug)]
pub struct WorkspaceCtx {
    abs_cwd: PathBuf,
    git_ctx: Result<Option<GitRepoCtx>, NewGitRepoCtxError>,
}

impl WorkspaceCtx {
    /// Create a new workspace context.
    pub fn new() -> Result<Self, NewWorkspaceError> {
        let abs_cwd = std::env::current_dir().map_err(NewWorkspaceError::GetCwdError)?;

        Ok(Self {
            git_ctx: GitRepoCtx::new(&abs_cwd),
            abs_cwd,
        })
    }

    /// Absolute path to the current working directory.
    pub fn cwd(&self) -> &Path {
        &self.abs_cwd
    }

    /// Grab a handle to the active git repo.
    ///
    /// Returns `Ok(Option::None)` if the cwd is not a git rpeo.
    /// Returns `Err(NewGitRepoCtxError)` if there was an error querying the git context.
    pub fn git_ctx(&self) -> Result<Option<&GitRepoCtx>, &NewGitRepoCtxError> {
        self.git_ctx.as_ref().map(Option::as_ref)
    }
}
