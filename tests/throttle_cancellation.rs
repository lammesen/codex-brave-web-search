use codex_brave_web_search::throttle::RequestThrottle;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

#[tokio::test]
async fn acquire_cancellable_returns_when_cancelled_while_waiting() {
    let throttle = Arc::new(RequestThrottle::new(1, 1));
    throttle.acquire().await;

    let cancelled = Arc::new(AtomicBool::new(false));
    let throttle_for_task = Arc::clone(&throttle);
    let cancelled_for_task = Arc::clone(&cancelled);

    let handle = tokio::spawn(async move {
        let is_cancelled = || cancelled_for_task.load(Ordering::Relaxed);
        throttle_for_task.acquire_cancellable(&is_cancelled).await
    });

    tokio::time::sleep(Duration::from_millis(40)).await;
    cancelled.store(true, Ordering::Relaxed);

    let joined = tokio::time::timeout(Duration::from_millis(300), handle)
        .await
        .expect("task should exit promptly")
        .expect("task should join");

    assert!(joined.is_err());
}

#[tokio::test]
async fn acquire_cancellable_succeeds_without_cancellation() {
    let throttle = RequestThrottle::new(10, 1);
    throttle.acquire().await;

    let is_cancelled = || false;
    let acquired = tokio::time::timeout(Duration::from_millis(300), async {
        throttle.acquire_cancellable(&is_cancelled).await
    })
    .await
    .expect("acquire should complete");

    assert!(acquired.is_ok());
}
