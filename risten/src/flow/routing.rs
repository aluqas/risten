//! Routing abstraction layer.
//!
//! This module provides a trait-based routing abstraction that allows
//! different routing backends (HashMap, phf, matchit) to be swapped
//! without changing application code.

/// Result of a routing lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteResult<'a, V> {
    /// Route matched, contains the value.
    Matched(&'a V),
    /// No matching route found.
    NotFound,
}

impl<'a, V> RouteResult<'a, V> {
    /// Returns true if the route was matched.
    pub fn is_matched(&self) -> bool {
        matches!(self, RouteResult::Matched(_))
    }

    /// Returns the matched value, if any.
    pub fn matched(self) -> Option<&'a V> {
        match self {
            RouteResult::Matched(v) => Some(v),
            RouteResult::NotFound => None,
        }
    }
}

/// A router that maps keys to values.
///
/// This trait abstracts over different routing implementations,
/// allowing the use of HashMap, phf, matchit, or custom routers.
pub trait Router<K: ?Sized, V>: Send + Sync + 'static {
    /// Look up a value by key.
    fn route(&self, key: &K) -> RouteResult<'_, V>;

    /// Check if a key exists in the router.
    fn contains(&self, key: &K) -> bool {
        self.route(key).is_matched()
    }
}

/// Error type for router building operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouterBuildError {
    /// A duplicate key was inserted.
    DuplicateKey(String),
    /// The router could not be built.
    BuildFailed(String),
}

impl std::fmt::Display for RouterBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RouterBuildError::DuplicateKey(key) => write!(f, "Duplicate key: {}", key),
            RouterBuildError::BuildFailed(msg) => write!(f, "Build failed: {}", msg),
        }
    }
}

impl std::error::Error for RouterBuildError {}

/// Builder for constructing routers.
///
/// This trait allows different router implementations to be built
/// using a common interface.
pub trait RouterBuilder<K, V>: Default + Send {
    /// The router type this builder produces.
    type Router: Router<K, V>;

    /// Insert a key-value pair into the router.
    ///
    /// Returns an error if the key already exists.
    fn insert(&mut self, key: K, value: V) -> Result<(), RouterBuildError>;

    /// Build the router, consuming the builder.
    fn build(self) -> Result<Self::Router, RouterBuildError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_result_helpers() {
        let val = 42;
        let matched = RouteResult::Matched(&val);
        let not_found: RouteResult<i32> = RouteResult::NotFound;

        assert!(matched.is_matched());
        assert!(!not_found.is_matched());

        assert_eq!(matched.matched(), Some(&42));
        assert_eq!(not_found.matched(), None);
    }

    #[test]
    fn test_router_build_error_display() {
        let err1 = RouterBuildError::DuplicateKey("key".to_string());
        let err2 = RouterBuildError::BuildFailed("reason".to_string());

        assert_eq!(format!("{}", err1), "Duplicate key: key");
        assert_eq!(format!("{}", err2), "Build failed: reason");
    }
}
