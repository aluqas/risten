//! Router abstraction for key-based dispatch.

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
pub trait RouterBuilder<K, V>: Default + Send {
    /// The router type this builder produces.
    type Router: Router<K, V>;

    /// Insert a key-value pair into the router.
    fn insert(&mut self, key: K, value: V) -> Result<(), RouterBuildError>;

    /// Build the router, consuming the builder.
    fn build(self) -> Result<Self::Router, RouterBuildError>;
}
