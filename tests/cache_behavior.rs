use codex_brave_web_search::cache::SearchCache;
use std::time::Duration;

#[tokio::test]
async fn insert_purges_expired_entries_for_other_keys() {
    let cache = SearchCache::new(Duration::from_millis(20));

    cache.insert("expired".to_string(), 1usize).await;
    tokio::time::sleep(Duration::from_millis(35)).await;

    cache.insert("fresh".to_string(), 2usize).await;

    assert_eq!(cache.get("expired").await, None);
    assert_eq!(cache.get("fresh").await, Some(2));
    assert_eq!(cache.len().await, 1);
}

#[tokio::test]
async fn insert_keeps_unexpired_entries() {
    let cache = SearchCache::new(Duration::from_millis(200));

    cache.insert("a".to_string(), 1usize).await;
    tokio::time::sleep(Duration::from_millis(10)).await;
    cache.insert("b".to_string(), 2usize).await;

    assert_eq!(cache.get("a").await, Some(1));
    assert_eq!(cache.get("b").await, Some(2));
    assert_eq!(cache.len().await, 2);
}
