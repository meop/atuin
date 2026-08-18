use std::sync::LazyLock;

use super::workspace::WorkspaceCtx;
use atuin_domain::{AtuinHostname, AtuinUsername};

/// Effectively-global application state, constructed once and held by the [`app()`](super::app)
/// static.
///
/// This is the single discoverable entry point for a process's ambient identity and location:
/// its [`session`](Self::session), [`cwd`](Self::cwd), [`hostname`](Self::hostname),
/// [`username`](Self::username), and [`workspace`](Self::workspace).
pub struct AppCtx {
    /// State on the current working directory.
    ///
    /// Constructed lazily on first [`workspace`](Self::workspace) access: many commands only need
    /// identity (session/host/user) and never touch the workspace, so they should not pay for its
    /// cwd resolution and background git discovery — nor its panic if the cwd is unreadable.
    workspace: LazyLock<WorkspaceCtx>,
}

impl AppCtx {
    pub(crate) fn new() -> Self {
        Self {
            workspace: LazyLock::new(WorkspaceCtx::new),
        }
    }

    /// Information held within the current working directory of atuin.
    ///
    /// The first call resolves the workspace (reads the cwd and kicks off background git
    /// discovery); subsequent calls are cheap.
    #[must_use]
    pub fn workspace(&self) -> &WorkspaceCtx {
        &self.workspace
    }

    /// The current session id, as exported by the shell integration in `ATUIN_SESSION`.
    ///
    /// [`None`] when the variable is unset (e.g. atuin invoked outside a hooked shell). Probed
    /// live, as the value is fixed for the life of a process but set by the environment.
    #[must_use]
    pub fn session(&self) -> Option<String> {
        std::env::var("ATUIN_SESSION").ok()
    }

    /// The current working directory as atuin records it.
    ///
    /// Prefers `$PWD` (preserving symlinks) and falls back to the physical cwd, matching how the
    /// rest of atuin resolves the recorded directory. Probed live, so it stays correct in
    /// long-running processes whose directory changes. This is distinct from
    /// [`WorkspaceCtx::cwd`], which is the physical root at which the workspace was resolved.
    #[must_use]
    pub fn cwd(&self) -> String {
        atuin_common::utils::get_current_dir()
    }

    /// The atuin-registered active hostname.
    ///
    /// Note that this always returns a new owned object as there is no way of knowing whether the
    /// hostname has changed or not at any given point.
    ///
    /// TODO(markovejnovic): A future implementation could have a refresh background task that
    ///                      refreshes the value periodically, avoiding an allocation.
    #[must_use]
    pub fn hostname(&self) -> AtuinHostname {
        AtuinHostname::probe()
    }

    /// The atuin-registered active username.
    ///
    /// Note that this always returns a new owned object as there is no way of knowing whether the
    /// hostname has changed or not at any given point.
    ///
    /// TODO(markovejnovic): A future implementation could have a refresh background task that
    ///                      refreshes the value periodically, avoiding an allocation.
    #[must_use]
    pub fn username(&self) -> AtuinUsername {
        AtuinUsername::probe()
    }
}
