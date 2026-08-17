use super::workspace::WorkspaceCtx;
use atuin_domain::{AtuinHostname, AtuinUsername};
use tracing::warn;

/// Effectively-global application state, constructed once and held by the [`app()`](super::app)
/// static.
pub struct AppCtx {
    /// State on the current working directory. [`Option::None`] if it fails to load.
    workspace: Option<WorkspaceCtx>,
}

impl AppCtx {
    pub(crate) fn new() -> Self {
        Self {
            workspace: WorkspaceCtx::new()
                .map(|e| {
                    warn!(err = e, "Failed to load the current workspace context");
                    e
                })
                .ok(),
        }
    }

    /// Information held within the current working directory of atuin.
    #[must_use]
    pub fn workspace(&self) -> Option<&WorkspaceCtx> {
        self.workspace.map(|f| &f)
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
