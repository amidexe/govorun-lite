//! Bind Core diagnostics to the managed call, without asynchronous journal writes.
use std::cell::RefCell;
tokio::task_local! { static CURRENT: RefCell<Option<String>>; }
pub(crate) fn record(value: String) {
    let _ = CURRENT.try_with(|slot| *slot.borrow_mut() = Some(value));
}
pub(crate) async fn capture<T>(action: impl std::future::Future<Output = T>) -> (T, Option<String>) {
    CURRENT.scope(RefCell::new(None), async {
        let value = action.await;
        (value, CURRENT.with(|slot| slot.borrow_mut().take()))
    }).await
}
