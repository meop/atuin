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
    #[must_use]
    pub fn hostname(&self) -> &AtuinHostname {
        // Note that this queries, unfortunately, all the time, since we never know when the
        // hostname could change from under us.
    }

    #[must_use]
    pub fn username(&self) -> &AtuinUsername {}
}
