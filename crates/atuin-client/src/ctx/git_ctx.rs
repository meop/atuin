use std::path::Path;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum NewGitRepoCtxError {
    #[error("failed to probe the git repo: {0}")]
    DiscoverGitRepo(gix::discover::Error),

    #[error("failed to open the main worktree's git repo: {0}")]
    OpenMainRepo(gix::open::Error),
}

/// A context handle for a particular git repo.
#[derive(Debug, Clone)]
pub struct GitRepoCtx {
    /// The [`gix::Repository`] initialized for the given absolute path.
    pub repo: gix::Repository,

    /// The main worktree's [`gix::Repository`].
    ///
    /// [`None`] when it is the same as [`Self::git_repo`] -- i.e. `git_repo` is the main
    /// worktree (or a repo with no linked worktrees) -- which avoids re-opening it. [`Some`]
    /// only for a linked worktree, where it holds the distinct main repository.
    main_repo: Option<gix::Repository>,
}

impl GitRepoCtx {
    pub fn new(path: &Path) -> Result<Option<Self>, NewGitRepoCtxError> {
        let repo = match gix::discover(path) {
            Ok(repo) => repo,
            // Not being inside a git repository is a normal state, not an error.
            Err(gix::discover::Error::Discover(
                gix::discover::upwards::Error::NoGitRepository { .. }
                | gix::discover::upwards::Error::NoGitRepositoryWithinCeiling { .. }
                | gix::discover::upwards::Error::NoGitRepositoryWithinFs { .. },
            )) => return Ok(None),
            Err(e) => return Err(NewGitRepoCtxError::DiscoverGitRepo(e)),
        };

        let main_repo = if repo.git_dir() == repo.common_dir() {
            None
        } else {
            Some(repo.main_repo().map_err(NewGitRepoCtxError::OpenMainRepo)?)
        };

        Ok(Some(Self { repo, main_repo }))
    }

    /// A handle to a [`gix::Repository`] of the main repo.
    ///
    /// If you are in a worktree, this returns the [`gix::Repository`] of that worktree, otherwise
    /// returns the [`gix::Repository`] of the current working directory.
    pub fn main_repo(&self) -> &gix::Repository {
        self.main_repo.as_ref().unwrap_or(&self.repo)
    }
}
