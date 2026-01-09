//! Dynamic routing implementations.
//!
//! These routers allow runtime modification and are explicitly in the `dynamic` module.

use risten_core::{RouteResult, Router, RouterBuildError, RouterBuilder};
use std::collections::HashMap;
use std::hash::Hash;

/// A HashMap-based router for dynamic key-value routing.
///
/// This router is in the `dynamic` module because it supports runtime insertion.
pub struct HashMapRouter<K, V> {
    map: HashMap<K, V>,
}

impl<K, V> HashMapRouter<K, V>
where
    K: Eq + Hash,
{
    /// Get a reference to the inner map.
    pub fn inner(&self) -> &HashMap<K, V> {
        &self.map
    }
}

impl<K, V> Router<K, V> for HashMapRouter<K, V>
where
    K: Eq + Hash + Send + Sync + 'static,
    V: Send + Sync + 'static,
{
    fn route(&self, key: &K) -> RouteResult<'_, V> {
        match self.map.get(key) {
            Some(v) => RouteResult::Matched(v),
            None => RouteResult::NotFound,
        }
    }
}

/// Builder for HashMapRouter.
pub struct HashMapRouterBuilder<K, V> {
    map: HashMap<K, V>,
}

impl<K, V> Default for HashMapRouterBuilder<K, V>
where
    K: Eq + Hash,
{
    fn default() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
}

impl<K, V> RouterBuilder<K, V> for HashMapRouterBuilder<K, V>
where
    K: Eq + Hash + Clone + ToString + Send + Sync + 'static,
    V: Send + Sync + 'static,
{
    type Router = HashMapRouter<K, V>;

    fn insert(&mut self, key: K, value: V) -> Result<(), RouterBuildError> {
        if self.map.contains_key(&key) {
            return Err(RouterBuildError::DuplicateKey(key.to_string()));
        }
        self.map.insert(key, value);
        Ok(())
    }

    fn build(self) -> Result<Self::Router, RouterBuildError> {
        Ok(HashMapRouter { map: self.map })
    }
}
