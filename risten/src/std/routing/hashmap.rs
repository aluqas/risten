//! HashMap-based router implementation.
//!
//! This is the default router that requires no external dependencies.

use std::{collections::HashMap, hash::Hash};

use crate::flow::routing::{RouteResult, Router, RouterBuildError, RouterBuilder};

/// A router backed by `HashMap`.
///
/// This is the default implementation that works with any hashable key type.
pub struct HashMapRouter<K, V> {
    map: HashMap<K, V>,
}

impl<K, V> HashMapRouter<K, V> {
    /// Create a new empty router.
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    /// Create a router from an existing HashMap.
    pub fn from_map(map: HashMap<K, V>) -> Self {
        Self { map }
    }

    /// Get the number of routes.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Check if the router is empty.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

impl<K, V> Default for HashMapRouter<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> Router<K, V> for HashMapRouter<K, V>
where
    K: Hash + Eq + Send + Sync + 'static,
    V: Send + Sync + 'static,
{
    fn route(&self, key: &K) -> RouteResult<'_, V> {
        match self.map.get(key) {
            Some(v) => RouteResult::Matched(v),
            None => RouteResult::NotFound,
        }
    }
}

/// Builder for `HashMapRouter`.
pub struct HashMapRouterBuilder<K, V> {
    map: HashMap<K, V>,
    allow_duplicates: bool,
}

impl<K, V> HashMapRouterBuilder<K, V> {
    /// Allow duplicate keys (later insertions override earlier ones).
    pub fn allow_duplicates(mut self) -> Self {
        self.allow_duplicates = true;
        self
    }
}

impl<K, V> Default for HashMapRouterBuilder<K, V> {
    fn default() -> Self {
        Self {
            map: HashMap::new(),
            allow_duplicates: false,
        }
    }
}

impl<K, V> RouterBuilder<K, V> for HashMapRouterBuilder<K, V>
where
    K: Hash + Eq + Send + Sync + std::fmt::Debug + 'static,
    V: Send + Sync + 'static,
{
    type Router = HashMapRouter<K, V>;

    fn insert(&mut self, key: K, value: V) -> Result<(), RouterBuildError> {
        if !self.allow_duplicates && self.map.contains_key(&key) {
            return Err(RouterBuildError::DuplicateKey(format!("{:?}", key)));
        }
        self.map.insert(key, value);
        Ok(())
    }

    fn build(self) -> Result<Self::Router, RouterBuildError> {
        Ok(HashMapRouter { map: self.map })
    }
}

#[cfg(test)]
mod tests {
    use super::{HashMapRouterBuilder, Router, RouterBuildError, RouterBuilder};

    #[test]
    fn test_basic_routing() {
        let mut builder: HashMapRouterBuilder<String, i32> = HashMapRouterBuilder::default();
        builder.insert("hello".to_string(), 1).unwrap();
        builder.insert("world".to_string(), 2).unwrap();

        let router = builder.build().unwrap();

        assert_eq!(router.route(&"hello".to_string()).matched(), Some(&1));
        assert_eq!(router.route(&"world".to_string()).matched(), Some(&2));
        assert_eq!(router.route(&"unknown".to_string()).matched(), None);
    }

    #[test]
    fn test_duplicate_key_error() {
        let mut builder: HashMapRouterBuilder<String, i32> = HashMapRouterBuilder::default();
        builder.insert("key".to_string(), 1).unwrap();

        let result = builder.insert("key".to_string(), 2);
        assert!(matches!(result, Err(RouterBuildError::DuplicateKey(_))));
    }

    #[test]
    fn test_allow_duplicates() {
        let mut builder: HashMapRouterBuilder<String, i32> =
            HashMapRouterBuilder::default().allow_duplicates();
        builder.insert("key".to_string(), 1).unwrap();
        builder.insert("key".to_string(), 2).unwrap();

        let router = builder.build().unwrap();
        assert_eq!(router.route(&"key".to_string()).matched(), Some(&2));
    }
}
