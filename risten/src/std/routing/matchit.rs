//! Matchit-based router implementation.
//!
//! Provides high-performance wildcard/parameter matching for paths (e.g. `events/:type/*`).

#[cfg(feature = "matchit")]
use crate::flow::routing::{RouteResult, Router, RouterBuildError, RouterBuilder};
#[cfg(feature = "matchit")]
use matchit::{Match, Router as InnerRouter};

/// A router based on `matchit`.
#[cfg(feature = "matchit")]
pub struct MatchitRouter<V> {
    router: InnerRouter<V>,
}

#[cfg(feature = "matchit")]
impl<V: Clone + Send + Sync + 'static> Router<str, V> for MatchitRouter<V> {
    fn route(&self, key: &str) -> RouteResult<'_, V> {
        match self.router.at(key) {
            Ok(Match { value, .. }) => RouteResult::Matched(value),
            Err(_) => RouteResult::NotFound,
        }
    }
}

#[cfg(feature = "matchit")]
impl<V: Clone + Send + Sync + 'static> Router<String, V> for MatchitRouter<V> {
    fn route(&self, key: &String) -> RouteResult<'_, V> {
        <Self as Router<str, V>>::route(self, key)
    }
}

/// Builder for MatchitRouter.
#[cfg(feature = "matchit")]
pub struct MatchitRouterBuilder<V> {
    router: InnerRouter<V>,
}

#[cfg(feature = "matchit")]
impl<V> Default for MatchitRouterBuilder<V> {
    fn default() -> Self {
        Self {
            router: InnerRouter::new(),
        }
    }
}

#[cfg(feature = "matchit")]
impl<V: Clone + Send + Sync + 'static> RouterBuilder<String, V> for MatchitRouterBuilder<V> {
    type Router = MatchitRouter<V>;

    fn insert(&mut self, key: String, value: V) -> Result<(), RouterBuildError> {
        self.router.insert(key.clone(), value).map_err(|e| {
            // matchit errors if route exists or is invalid
            RouterBuildError::DuplicateKey(format!("{}: {}", key, e))
        })
    }

    fn build(self) -> Result<Self::Router, RouterBuildError> {
        Ok(MatchitRouter {
            router: self.router,
        })
    }
}
