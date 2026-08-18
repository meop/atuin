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
/// begins driving it immediately. Retrieve the result with [`get`](Self::get), which will either:
///
///   - Wait for the future to land with the result, returning the result.
///   - Immediately retrieve the computation if it is successfully cached.
///
/// This utility should be used within a Tokio runtime. If constructed outside a Tokio runtime, the
/// eager load within [`EagerFutureCell::new`] is skipped and the work is performed lazily on the
/// first [`Self::get`] call.
pub struct EagerFutureCell<T> {
    cell: Arc<OnceCell<T>>,
    driver: Driver,
}

impl<T: Send + Sync + 'static> EagerFutureCell<T> {
    /// Initialize the cell with the given `work` future, starting the work immediately if a tokio
    /// runtime is available.
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

    /// Fetch the value, awaiting the work if necessary.
    pub async fn get(&self) -> &T {
        self.driver.clone().await;
        self.cell
            .get()
            .expect("cell value was never populated. likely the work future panicked.")
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
