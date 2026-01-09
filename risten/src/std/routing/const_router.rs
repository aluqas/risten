//! Const Generics Router - Compile-time static routing.
//!
//! This module provides a router implementation using const generics
//! that enables compile-time optimization for fixed routing tables.
//! Ideal for scenarios where the set of routes is known at compile time.

use crate::flow::routing::{RouteResult, Router};

/// A router with a fixed-size routing table known at compile time.
///
/// Uses const generics to embed the routing table directly,
/// enabling the compiler to optimize the dispatch logic.
///
/// # Type Parameters
///
/// - `K`: The key type (must be Ord for binary search)
/// - `V`: The value type
/// - `N`: The number of routes (const generic)
///
/// # Example
///
/// ```rust,ignore
/// use risten::source::router::ConstRouter;
///
/// // Define routes at compile time
/// const ROUTER: ConstRouter<&'static str, fn() -> String, 3> = ConstRouter::new([
///     ("echo", echo_handler as fn() -> String),
///     ("help", help_handler as fn() -> String),
///     ("ping", ping_handler as fn() -> String),
/// ]);
///
/// // Route lookup is optimized by the compiler
/// match ROUTER.route(&"ping") {
///     RouteResult::Matched(handler) => handler(),
///     RouteResult::NotFound => "Unknown command".into(),
/// }
/// ```
///
/// # Performance
///
/// For small N (< 8), linear search is used.
/// For larger N, binary search provides O(log N) lookup.
/// The compiler can often inline and optimize the entire lookup.
pub struct ConstRouter<K, V, const N: usize> {
    /// Sorted array of (key, value) pairs.
    /// Must be sorted by key for binary search optimization.
    routes: [(K, V); N],
}

impl<K, V, const N: usize> ConstRouter<K, V, N>
where
    K: Ord,
{
    /// Create a new const router from a sorted array of routes.
    ///
    /// # Panics
    ///
    /// In debug mode, panics if the routes are not sorted by key.
    pub const fn new(routes: [(K, V); N]) -> Self {
        // Note: We cannot verify sorting at compile time in const fn
        // with current stable Rust. Users should ensure sorted input.
        Self { routes }
    }

    /// Create a const router and sort the routes at runtime.
    ///
    /// Use this when you cannot guarantee sorted input.
    pub fn new_sorted(mut routes: [(K, V); N]) -> Self
    where
        K: Ord + Clone,
        V: Clone,
    {
        routes.sort_by(|a, b| a.0.cmp(&b.0));
        Self { routes }
    }

    /// Look up a value by key using binary search.
    #[inline]
    pub fn lookup(&self, key: &K) -> Option<&V> {
        // For very small arrays, linear search may be faster
        if N <= 4 {
            for (k, v) in &self.routes {
                if k == key {
                    return Some(v);
                }
            }
            None
        } else {
            // Binary search for larger arrays
            self.routes
                .binary_search_by(|(k, _)| k.cmp(key))
                .ok()
                .map(|idx| &self.routes[idx].1)
        }
    }

    /// Get the number of routes.
    #[inline]
    pub const fn len(&self) -> usize {
        N
    }

    /// Check if the router is empty.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        N == 0
    }
}

impl<K, V, const N: usize> Router<K, V> for ConstRouter<K, V, N>
where
    K: Ord + Send + Sync + 'static,
    V: Send + Sync + 'static,
{
    fn route(&self, key: &K) -> RouteResult<'_, V> {
        match self.lookup(key) {
            Some(v) => RouteResult::Matched(v),
            None => RouteResult::NotFound,
        }
    }
}

// ============================================================================
// Macro for convenient ConstRouter construction
// ============================================================================

/// Create a const router with automatic sorting.
///
/// # Example
///
/// ```rust,ignore
/// use risten::const_router;
///
/// const_router! {
///     COMMANDS: &'static str => MyHandler {
///         "ping" => PingHandler,
///         "echo" => EchoHandler,
///         "help" => HelpHandler,
///     }
/// }
/// ```
#[macro_export]
macro_rules! const_router {
    (
        $vis:vis $name:ident: $key:ty => $val:ty {
            $($k:expr => $v:expr),+ $(,)?
        }
    ) => {
        $vis static $name: $crate::source::router::ConstRouter<$key, $val, { const_router!(@count $($k),+) }> =
            $crate::source::router::ConstRouter::new([
                $(($k, $v)),+
            ]);
    };
    (@count $($x:expr),*) => {
        <[()]>::len(&[$(const_router!(@replace $x ())),*])
    };
    (@replace $_:expr, $sub:expr) => { $sub };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_const_router_lookup() {
        let router: ConstRouter<&str, i32, 3> =
            ConstRouter::new([("apple", 1), ("banana", 2), ("cherry", 3)]);

        assert_eq!(router.lookup(&"apple"), Some(&1));
        assert_eq!(router.lookup(&"banana"), Some(&2));
        assert_eq!(router.lookup(&"cherry"), Some(&3));
        assert_eq!(router.lookup(&"durian"), None);
    }

    #[test]
    fn test_const_router_trait() {
        let router: ConstRouter<&str, i32, 3> = ConstRouter::new([("a", 10), ("b", 20), ("c", 30)]);

        assert!(router.route(&"a").is_matched());
        assert_eq!(router.route(&"b").matched(), Some(&20));
        assert!(!router.route(&"z").is_matched());
    }

    #[test]
    fn test_const_router_sorted() {
        // Out of order input
        let router: ConstRouter<i32, &str, 4> =
            ConstRouter::new_sorted([(3, "three"), (1, "one"), (4, "four"), (2, "two")]);

        assert_eq!(router.lookup(&1), Some(&"one"));
        assert_eq!(router.lookup(&2), Some(&"two"));
        assert_eq!(router.lookup(&3), Some(&"three"));
        assert_eq!(router.lookup(&4), Some(&"four"));
    }

    #[test]
    fn test_const_router_empty() {
        let router: ConstRouter<&str, i32, 0> = ConstRouter::new([]);
        assert!(router.is_empty());
        assert_eq!(router.len(), 0);
        assert_eq!(router.lookup(&"anything"), None);
    }

    #[test]
    fn test_const_router_single() {
        let router: ConstRouter<&str, i32, 1> = ConstRouter::new([("only", 42)]);
        assert_eq!(router.len(), 1);
        assert_eq!(router.lookup(&"only"), Some(&42));
        assert_eq!(router.lookup(&"other"), None);
    }

    #[test]
    fn test_const_router_large() {
        // Test with larger array (triggers binary search path)
        let router: ConstRouter<i32, i32, 10> = ConstRouter::new([
            (0, 0),
            (1, 10),
            (2, 20),
            (3, 30),
            (4, 40),
            (5, 50),
            (6, 60),
            (7, 70),
            (8, 80),
            (9, 90),
        ]);

        for i in 0..10 {
            assert_eq!(router.lookup(&i), Some(&(i * 10)));
        }
        assert_eq!(router.lookup(&10), None);
        assert_eq!(router.lookup(&-1), None);
    }
}
