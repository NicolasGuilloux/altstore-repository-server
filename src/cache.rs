use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Cache key combining file path and modification timestamp
/// The timestamp ensures cache invalidation when file is modified
pub type CacheKey = (PathBuf, u64);

/// Cached IPA metadata extracted from Info.plist
#[derive(Debug, Clone)]
pub struct CachedIpaInfo {
    pub bundle_identifier: String,
    pub bundle_version: String,
    pub bundle_short_version: Option<String>,
    pub bundle_name: String,
}

/// Thread-safe cache for IPA metadata
/// Uses Arc<RwLock<HashMap>> for concurrent read access with exclusive writes
#[derive(Clone)]
pub struct IpaCache {
    inner: Arc<RwLock<HashMap<CacheKey, CachedIpaInfo>>>,
}

impl IpaCache {
    /// Creates a new empty cache
    pub fn new() -> Self {
        tracing::debug!("Creating new IPA metadata cache");
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Retrieves cached metadata for a given path and modification timestamp
    /// Returns None if not found (cache miss)
    pub async fn get(&self, key: &CacheKey) -> Option<CachedIpaInfo> {
        let cache = self.inner.read().await;
        let result = cache.get(key).cloned();

        match &result {
            Some(_) => {
                tracing::debug!(
                    path = %key.0.display(),
                    mtime = key.1,
                    "Cache hit for IPA metadata"
                );
            }
            None => {
                tracing::debug!(
                    path = %key.0.display(),
                    mtime = key.1,
                    "Cache miss for IPA metadata"
                );
            }
        }

        result
    }

    /// Inserts metadata into the cache
    pub async fn insert(&self, key: CacheKey, value: CachedIpaInfo) {
        tracing::debug!(
            path = %key.0.display(),
            mtime = key.1,
            bundle_id = %value.bundle_identifier,
            "Caching IPA metadata"
        );

        let mut cache = self.inner.write().await;
        cache.insert(key, value);
    }
}

impl Default for IpaCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_miss() {
        let cache = IpaCache::new();
        let key = (PathBuf::from("/test/app.ipa"), 12345);
        assert!(cache.get(&key).await.is_none());
    }

    #[tokio::test]
    async fn test_cache_hit() {
        let cache = IpaCache::new();
        let key = (PathBuf::from("/test/app.ipa"), 12345);
        let info = CachedIpaInfo {
            bundle_identifier: "com.example.app".to_string(),
            bundle_version: "1.0.0".to_string(),
            bundle_short_version: Some("1.0".to_string()),
            bundle_name: "TestApp".to_string(),
        };

        cache.insert(key.clone(), info.clone()).await;

        let result = cache.get(&key).await;
        assert!(result.is_some());
        let cached = result.unwrap();
        assert_eq!(cached.bundle_identifier, "com.example.app");
        assert_eq!(cached.bundle_version, "1.0.0");
    }

    #[tokio::test]
    async fn test_different_mtime_is_cache_miss() {
        let cache = IpaCache::new();
        let key1 = (PathBuf::from("/test/app.ipa"), 12345);
        let key2 = (PathBuf::from("/test/app.ipa"), 67890);

        let info = CachedIpaInfo {
            bundle_identifier: "com.example.app".to_string(),
            bundle_version: "1.0.0".to_string(),
            bundle_short_version: None,
            bundle_name: "TestApp".to_string(),
        };

        cache.insert(key1, info).await;

        // Same path but different mtime should be a miss
        assert!(cache.get(&key2).await.is_none());
    }

    #[tokio::test]
    async fn test_cache_insert_and_get() {
        let cache = IpaCache::new();
        let path = PathBuf::from("/apps/TestApp/test_1.0.0.ipa");
        let mtime = 1700000000u64;
        let key = (path.clone(), mtime);

        let info = CachedIpaInfo {
            bundle_identifier: "com.test.testapp".to_string(),
            bundle_version: "100".to_string(),
            bundle_short_version: Some("1.0.0".to_string()),
            bundle_name: "Test App".to_string(),
        };

        // Insert into cache
        cache.insert(key.clone(), info).await;

        // Retrieve from cache
        let result = cache.get(&key).await;
        assert!(result.is_some(), "Expected cache hit after insert");

        let cached = result.unwrap();
        assert_eq!(cached.bundle_identifier, "com.test.testapp");
        assert_eq!(cached.bundle_version, "100");
        assert_eq!(cached.bundle_short_version, Some("1.0.0".to_string()));
        assert_eq!(cached.bundle_name, "Test App");
    }

    #[tokio::test]
    async fn test_cache_miss_on_different_mtime() {
        let cache = IpaCache::new();
        let path = PathBuf::from("/apps/MyApp/app_2.0.0.ipa");
        let original_mtime = 1600000000u64;
        let updated_mtime = 1600001000u64; // File was modified

        let original_key = (path.clone(), original_mtime);
        let updated_key = (path.clone(), updated_mtime);

        let info = CachedIpaInfo {
            bundle_identifier: "com.example.myapp".to_string(),
            bundle_version: "200".to_string(),
            bundle_short_version: Some("2.0.0".to_string()),
            bundle_name: "My App".to_string(),
        };

        // Insert with original mtime
        cache.insert(original_key.clone(), info).await;

        // Verify original key still works
        assert!(
            cache.get(&original_key).await.is_some(),
            "Original key should still hit"
        );

        // Verify different mtime causes cache miss (simulates file modification)
        assert!(
            cache.get(&updated_key).await.is_none(),
            "Different mtime should cause cache miss"
        );
    }

    #[tokio::test]
    async fn test_cache_key_with_same_path_different_mtime() {
        let cache = IpaCache::new();
        let path = PathBuf::from("/apps/SharedApp/shared_1.0.0.ipa");

        // Two different modification times for the same path
        let mtime_v1 = 1500000000u64;
        let mtime_v2 = 1500500000u64;

        let key_v1 = (path.clone(), mtime_v1);
        let key_v2 = (path.clone(), mtime_v2);

        let info_v1 = CachedIpaInfo {
            bundle_identifier: "com.shared.app".to_string(),
            bundle_version: "100".to_string(),
            bundle_short_version: Some("1.0.0".to_string()),
            bundle_name: "Shared App v1".to_string(),
        };

        let info_v2 = CachedIpaInfo {
            bundle_identifier: "com.shared.app".to_string(),
            bundle_version: "110".to_string(),
            bundle_short_version: Some("1.1.0".to_string()),
            bundle_name: "Shared App v2".to_string(),
        };

        // Insert both versions
        cache.insert(key_v1.clone(), info_v1).await;
        cache.insert(key_v2.clone(), info_v2).await;

        // Both should be retrievable with their respective keys
        let result_v1 = cache.get(&key_v1).await;
        let result_v2 = cache.get(&key_v2).await;

        assert!(result_v1.is_some(), "v1 key should return cached data");
        assert!(result_v2.is_some(), "v2 key should return cached data");

        // Verify each key returns the correct version
        let cached_v1 = result_v1.unwrap();
        let cached_v2 = result_v2.unwrap();

        assert_eq!(cached_v1.bundle_version, "100");
        assert_eq!(cached_v1.bundle_name, "Shared App v1");

        assert_eq!(cached_v2.bundle_version, "110");
        assert_eq!(cached_v2.bundle_name, "Shared App v2");
    }
}
