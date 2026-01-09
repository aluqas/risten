//! Trie-based router for efficient string key routing.
//!
//! This module provides a Trie (prefix tree) based router that offers
//! O(key length) lookup performance, making it ideal for command routing
//! and path-based dispatching.
//!
//! # Overview
//!
//! Unlike `HashMapRouter` which uses hash-based O(1) lookup, `TrieRouter`
//! uses a prefix tree structure that:
//!
//! - Provides O(k) lookup where k is the key length
//! - Supports prefix-based matching
//! - Is memory-efficient for keys with common prefixes
//! - Enables substring and wildcard matching (future)
//!
//! # Example
//!
//! ```rust,ignore
//! use risten::routing::TrieRouter;
//!
//! let mut router = TrieRouter::new();
//! router.insert("ping", ping_handler);
//! router.insert("pong", pong_handler);
//! router.insert("help", help_handler);
//!
//! // O(key length) lookup
//! let handler = router.get("ping");
//! ```

use std::collections::HashMap;

/// A node in the Trie structure.
#[derive(Debug)]
struct TrieNode<V> {
    /// Children nodes, keyed by character.
    children: HashMap<char, TrieNode<V>>,
    /// The value stored at this node (if this is a terminal node).
    value: Option<V>,
}

impl<V> Default for TrieNode<V> {
    fn default() -> Self {
        Self {
            children: HashMap::new(),
            value: None,
        }
    }
}

impl<V> TrieNode<V> {
    /// Create a new empty Trie node.
    fn new() -> Self {
        Self::default()
    }
}

/// A Trie-based router for string key routing.
///
/// Provides O(key length) lookup performance using a prefix tree structure.
///
/// # Type Parameters
///
/// - `V`: The handler/value type stored for each route
///
/// # Example
///
/// ```rust
/// use risten::routing::TrieRouter;
///
/// let mut router: TrieRouter<&str> = TrieRouter::new();
/// router.insert("hello", "world");
/// router.insert("help", "me");
///
/// assert_eq!(router.get("hello"), Some(&"world"));
/// assert_eq!(router.get("help"), Some(&"me"));
/// assert_eq!(router.get("hell"), None);
/// ```
#[derive(Debug)]
pub struct TrieRouter<V> {
    root: TrieNode<V>,
    size: usize,
}

impl<V> Default for TrieRouter<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V> TrieRouter<V> {
    /// Create a new empty Trie router.
    pub fn new() -> Self {
        Self {
            root: TrieNode::new(),
            size: 0,
        }
    }

    /// Insert a key-value pair into the router.
    ///
    /// If the key already exists, the old value is replaced.
    ///
    /// # Arguments
    ///
    /// * `key` - The routing key (string)
    /// * `value` - The handler/value to associate with this key
    ///
    /// # Returns
    ///
    /// The old value if the key already existed, `None` otherwise.
    pub fn insert(&mut self, key: &str, value: V) -> Option<V> {
        let mut node = &mut self.root;

        for ch in key.chars() {
            node = node.children.entry(ch).or_insert_with(TrieNode::new);
        }

        let old = node.value.take();
        node.value = Some(value);

        if old.is_none() {
            self.size += 1;
        }

        old
    }

    /// Look up a value by exact key match.
    ///
    /// # Arguments
    ///
    /// * `key` - The routing key to look up
    ///
    /// # Returns
    ///
    /// A reference to the value if found, `None` otherwise.
    pub fn get(&self, key: &str) -> Option<&V> {
        let mut node = &self.root;

        for ch in key.chars() {
            match node.children.get(&ch) {
                Some(child) => node = child,
                None => return None,
            }
        }

        node.value.as_ref()
    }

    /// Look up a mutable value by exact key match.
    pub fn get_mut(&mut self, key: &str) -> Option<&mut V> {
        let mut node = &mut self.root;

        for ch in key.chars() {
            match node.children.get_mut(&ch) {
                Some(child) => node = child,
                None => return None,
            }
        }

        node.value.as_mut()
    }

    /// Check if a key exists in the router.
    pub fn contains(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    /// Get the number of keys in the router.
    pub fn len(&self) -> usize {
        self.size
    }

    /// Check if the router is empty.
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// Find the longest prefix match for a given key.
    ///
    /// This is useful for command routing where you want to match
    /// the most specific handler.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to find a prefix match for
    ///
    /// # Returns
    ///
    /// A tuple of (matched_prefix_length, value) if found.
    pub fn longest_prefix_match(&self, key: &str) -> Option<(usize, &V)> {
        let mut node = &self.root;
        let mut last_match: Option<(usize, &V)> = None;
        let mut depth = 0;

        for ch in key.chars() {
            match node.children.get(&ch) {
                Some(child) => {
                    node = child;
                    depth += 1;
                    if let Some(ref val) = node.value {
                        last_match = Some((depth, val));
                    }
                }
                None => break,
            }
        }

        last_match
    }
}

// ============================================================================
// Router Trait Implementation
// ============================================================================

use crate::flow::routing::{RouteResult, Router};

impl<K, V> Router<K, V> for TrieRouter<V>
where
    K: AsRef<str>,
    V: Send + Sync + 'static,
{
    fn route(&self, key: &K) -> RouteResult<'_, V> {
        match self.get(key.as_ref()) {
            Some(v) => RouteResult::Matched(v),
            None => RouteResult::NotFound,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_router_is_empty() {
        let router: TrieRouter<i32> = TrieRouter::new();
        assert!(router.is_empty());
        assert_eq!(router.len(), 0);
    }

    #[test]
    fn test_insert_and_get() {
        let mut router = TrieRouter::new();
        router.insert("hello", 1);
        router.insert("help", 2);
        router.insert("world", 3);

        assert_eq!(router.get("hello"), Some(&1));
        assert_eq!(router.get("help"), Some(&2));
        assert_eq!(router.get("world"), Some(&3));
        assert_eq!(router.len(), 3);
    }

    #[test]
    fn test_get_not_found() {
        let mut router = TrieRouter::new();
        router.insert("hello", 1);

        assert_eq!(router.get("hell"), None);
        assert_eq!(router.get("helloworld"), None);
        assert_eq!(router.get("bye"), None);
    }

    #[test]
    fn test_insert_replaces_value() {
        let mut router = TrieRouter::new();
        assert_eq!(router.insert("key", 1), None);
        assert_eq!(router.insert("key", 2), Some(1));
        assert_eq!(router.get("key"), Some(&2));
        assert_eq!(router.len(), 1); // Size unchanged
    }

    #[test]
    fn test_contains() {
        let mut router = TrieRouter::new();
        router.insert("ping", ());

        assert!(router.contains("ping"));
        assert!(!router.contains("pong"));
        assert!(!router.contains("pin"));
    }

    #[test]
    fn test_longest_prefix_match() {
        let mut router = TrieRouter::new();
        router.insert("a", 1);
        router.insert("ab", 2);
        router.insert("abc", 3);

        // Exact match
        assert_eq!(router.longest_prefix_match("abc"), Some((3, &3)));
        // Partial match
        assert_eq!(router.longest_prefix_match("abcd"), Some((3, &3)));
        assert_eq!(router.longest_prefix_match("ab"), Some((2, &2)));
        // No match
        assert_eq!(router.longest_prefix_match("xyz"), None);
    }

    #[test]
    fn test_common_prefix_sharing() {
        let mut router = TrieRouter::new();
        router.insert("ping", "pong");
        router.insert("pong", "ping");
        router.insert("help", "info");
        router.insert("hello", "world");

        assert_eq!(router.get("ping"), Some(&"pong"));
        assert_eq!(router.get("pong"), Some(&"ping"));
        assert_eq!(router.get("help"), Some(&"info"));
        assert_eq!(router.get("hello"), Some(&"world"));
    }

    #[test]
    fn test_router_trait() {
        let mut router = TrieRouter::new();
        router.insert("key", 42);

        // Use via Router trait
        let key = "key".to_string();
        assert_eq!(
            Router::<String, i32>::route(&router, &key),
            RouteResult::Matched(&42)
        );
    }

    #[test]
    fn test_unicode_keys() {
        let mut router = TrieRouter::new();
        router.insert("こんにちは", 1);
        router.insert("你好", 2);
        router.insert("🎉", 3);

        assert_eq!(router.get("こんにちは"), Some(&1));
        assert_eq!(router.get("你好"), Some(&2));
        assert_eq!(router.get("🎉"), Some(&3));
    }
}
