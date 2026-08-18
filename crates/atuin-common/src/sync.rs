//! Synchronization primitives.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use futures::FutureExt;
use futures::future::Shared;
use tokio::runtime::Handle;
use tokio::sync::OnceCell;

/// The work-driving future, type-erased so the cell holds a concrete type. Its `()` output (rather
/// than `T`) is what lets [`Shared`] apply even when `T` is not `Clone` — the value itself lives in
/// the [`OnceCell`].
type Driver = Shared<Pin<Box<dyn Future<Output = ()> + Send>>>;

/// A cell whose value is produced exactly once, eagerly, in the background.
///
/// [`EagerFutureCell::new`] takes an `async` computation and, if a Tokio runtime is available,
/// begins driving it immediately — so it overlaps with whatever else the caller does before the
/// value is needed. Retrieve the result with [`get`](Self::get), which awaits the in-flight
/// computation and returns a shared reference to the cached value; concurrent callers all await the
/// same computation, which runs at most once.
///
/// If constructed outside a Tokio runtime (for example in a synchronous test), the eager kick is
/// skipped and the work runs lazily on the first [`get`](Self::get) instead — so the cell is always
/// usable and never panics on construction.
///
/// The computation is async, so the caller chooses how it runs: a naturally-async computation can
/// be awaited directly, while blocking work should wrap itself in [`tokio::task::spawn_blocking`]
/// to stay off the executor.
pub struct EagerFutureCell<T> {
    cell: Arc<OnceCell<T>>,
    driver: Driver,
}

impl<T: Send + Sync + 'static> EagerFutureCell<T> {
    /// Create the cell from an `async` computation and, if a Tokio runtime is available, begin
    /// driving it in the background right away.
    pub fn new<Fut>(work: Fut) -> Self
    where
        Fut: Future<Output = T> + Send + 'static,
    {
        let cell: Arc<OnceCell<T>> = Arc::new(OnceCell::new());

        // The driver runs `work` once and stores its result. Sharing the driver (not `work`) lets
        // the eager task and every `get` await the same single execution.
        let driver: Driver = {
            let cell = cell.clone();
            async move {
                let _ = cell.set(work.await);
            }
            .boxed()
            .shared()
        };

        if let Ok(handle) = Handle::try_current() {
            handle.spawn(driver.clone());
        }

        Self { cell, driver }
    }

    /// Await the value, driving the computation first if the eager kick was skipped or has not yet
    /// finished. Cheap once resolved.
    pub async fn get(&self) -> &T {
        self.driver.clone().await;
        self.cell
            .get()
            .expect("driver resolved without populating the cell")
    }
}

#[cfg(test)]
mod tests {
    use super::EagerFutureCell;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn computes_once_and_caches() {
        let calls = Arc::new(AtomicUsize::new(0));
        let counter = calls.clone();
        let cell = EagerFutureCell::new(async move {
            counter.fetch_add(1, Ordering::SeqCst);
            42usize
        });

        // Repeated gets return the cached value, and the work runs exactly once even though the
        // eager background kick and these gets can race.
        assert_eq!(*cell.get().await, 42);
        assert_eq!(*cell.get().await, 42);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn falls_back_to_lazy_without_a_runtime() {
        // Constructed outside any Tokio runtime: the eager kick is skipped (no panic), yet the
        // value is still computed lazily on first `get`.
        let calls = Arc::new(AtomicUsize::new(0));
        let counter = calls.clone();
        let cell = EagerFutureCell::new(async move {
            counter.fetch_add(1, Ordering::SeqCst);
            7usize
        });

        let rt = tokio::runtime::Runtime::new().unwrap();
        assert_eq!(*rt.block_on(cell.get()), 7);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
