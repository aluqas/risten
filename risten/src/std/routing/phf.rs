//! PHF-based router implementation.
//!
//! Provides compile-time perfect hash map routing.
//! This router is immutable and must be constructed with a static map reference.

#[cfg(feature = "phf")]
use crate::flow::routing::{RouteResult, Router};

/// A router based on `phf::Map`.
///
/// Wraps a static reference to a PHF map.
#[cfg(feature = "phf")]
pub struct PhfRouter<V: 'static> {
    map: &'static phf::Map<&'static str, V>,
}

#[cfg(feature = "phf")]
impl<V: Send + Sync + 'static> PhfRouter<V> {
    /// Create a new router from a static PHF map.
    pub const fn new(map: &'static phf::Map<&'static str, V>) -> Self {
        Self { map }
    }
}

#[cfg(feature = "phf")]
impl<V: Send + Sync + 'static> Router<str, V> for PhfRouter<V> {
    fn route(&self, key: &str) -> RouteResult<'_, V> {
        match self.map.get(key) {
            Some(v) => RouteResult::Matched(v),
            None => RouteResult::NotFound,
        }
    }
}

// Note: RouterBuilder is not implemented for PhfRouter because PHF maps
// are constructed at compile time, not runtime.
