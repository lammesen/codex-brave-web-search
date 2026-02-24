use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
struct CacheEntry<T> {
    inserted_at: Instant,
    value: T,
}

#[derive(Debug)]
pub struct SearchCache<T> {
    ttl: Duration,
    entries: tokio::sync::RwLock<HashMap<String, CacheEntry<T>>>,
}

impl<T: Clone> SearchCache<T> {
    #[must_use]
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            entries: tokio::sync::RwLock::new(HashMap::new()),
        }
    }

    pub async fn get(&self, key: &str) -> Option<T> {
        let now = Instant::now();
        {
            let entries = self.entries.read().await;
            let entry = entries.get(key)?;
            if now.duration_since(entry.inserted_at) < self.ttl {
                return Some(entry.value.clone());
            }
        }

        let mut entries = self.entries.write().await;
        if let Some(entry) = entries.get(key)
            && now.duration_since(entry.inserted_at) >= self.ttl
        {
            entries.remove(key);
        }
        None
    }

    pub async fn insert(&self, key: String, value: T) {
        let now = Instant::now();
        let mut entries = self.entries.write().await;
        purge_expired_entries(&mut entries, now, self.ttl);
        entries.insert(
            key,
            CacheEntry {
                inserted_at: now,
                value,
            },
        );
    }

    pub async fn purge_expired(&self) {
        let now = Instant::now();
        let mut entries = self.entries.write().await;
        purge_expired_entries(&mut entries, now, self.ttl);
    }

    pub async fn len(&self) -> usize {
        self.entries.read().await.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.len().await == 0
    }
}

fn purge_expired_entries<T>(
    entries: &mut HashMap<String, CacheEntry<T>>,
    now: Instant,
    ttl: Duration,
) {
    entries.retain(|_, entry| now.duration_since(entry.inserted_at) < ttl);
}
