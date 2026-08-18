//! An eagerly-evaluated, shareable async cell for expensive blocking work.

use std::sync::Arc;

use tokio::runtime::Handle;
use tokio::sync::OnceCell;
use tokio::task;

/// A cell whose value is computed exactly once, eagerly, in the background.
///
/// [`EagerFutureCell::new`] kicks the (blocking) work off on the Tokio blocking pool immediately,
/// so it overlaps with whatever else the caller does before the value is needed. Retrieve the
/// result with [`get`](Self::get), which awaits the in-flight computation and then returns a shared
/// reference to the cached value; concurrent callers all await the same computation.
///
/// If constructed outside a Tokio runtime (for example in a synchronous test), the eager kick is
/// skipped and the work runs lazily on the first [`get`](Self::get) instead — so the cell is always
/// usable, never panics on construction, and still computes its value at most once.
pub struct EagerFutureCell<T> {
    cell: Arc<OnceCell<T>>,
    work: Arc<dyn Fn() -> T + Send + Sync>,
}

impl<T: Send + Sync + 'static> EagerFutureCell<T> {
    /// Create the cell and, if a Tokio runtime is available, begin computing `work` in the
    /// background right away.
    ///
    /// `work` is a *blocking* closure (it runs on [`tokio::task::spawn_blocking`]); it must be
    /// callable more than once so both the eager task and a lazy [`get`](Self::get) can supply it
    /// to the underlying [`OnceCell`], though it is only ever invoked once.
    pub fn new<F>(work: F) -> Self
    where
        F: Fn() -> T + Send + Sync + 'static,
    {
        let cell = Arc::new(OnceCell::new());
        let work: Arc<dyn Fn() -> T + Send + Sync> = Arc::new(work);

        if let Ok(handle) = Handle::try_current() {
            let cell = cell.clone();
            let work = work.clone();
            handle.spawn(async move {
                Self::init(&cell, &work).await;
            });
        }

        Self { cell, work }
    }

    /// Drive the one-shot initialization. Funnelling both the eager task and [`get`](Self::get)
    /// through [`OnceCell::get_or_init`] guarantees `work` runs exactly once even if they race.
    async fn init<'a>(cell: &'a OnceCell<T>, work: &Arc<dyn Fn() -> T + Send + Sync>) -> &'a T {
        cell.get_or_init(|| {
            let work = work.clone();
            async move {
                task::spawn_blocking(move || work())
                    .await
                    .expect("EagerFutureCell work panicked")
            }
        })
        .await
    }

    /// Await the value, computing it first if the eager kick was skipped or has not yet finished.
    /// Cheap once resolved.
    pub async fn get(&self) -> &T {
        Self::init(&self.cell, &self.work).await
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
        let cell = EagerFutureCell::new(move || {
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
        let cell = EagerFutureCell::new(move || {
            counter.fetch_add(1, Ordering::SeqCst);
            7usize
        });

        let rt = tokio::runtime::Runtime::new().unwrap();
        assert_eq!(*rt.block_on(cell.get()), 7);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
