//! Trie-based router for prefix matching.

use risten_core::{RouteResult, Router};
use std::collections::HashMap;

/// A trie node for string-based routing.
struct TrieNode<V> {
    value: Option<V>,
    children: HashMap<char, TrieNode<V>>,
}

impl<V> Default for TrieNode<V> {
    fn default() -> Self {
        Self {
            value: None,
            children: HashMap::new(),
        }
    }
}

/// A trie-based router for string keys.
///
/// Supports exact match and longest prefix match.
pub struct TrieRouter<V> {
    root: TrieNode<V>,
}

impl<V> Default for TrieRouter<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V> TrieRouter<V> {
    /// Create a new empty trie router.
    pub fn new() -> Self {
        Self {
            root: TrieNode::default(),
        }
    }

    /// Insert a key-value pair.
    pub fn insert(&mut self, key: &str, value: V) {
        let mut node = &mut self.root;
        for c in key.chars() {
            node = node.children.entry(c).or_default();
        }
        node.value = Some(value);
    }

    /// Find the longest prefix match.
    pub fn longest_prefix_match(&self, key: &str) -> Option<&V> {
        let mut node = &self.root;
        let mut last_match = node.value.as_ref();

        for c in key.chars() {
            match node.children.get(&c) {
                Some(child) => {
                    node = child;
                    if node.value.is_some() {
                        last_match = node.value.as_ref();
                    }
                }
                None => break,
            }
        }

        last_match
    }
}

impl<V: Send + Sync + 'static> Router<str, V> for TrieRouter<V> {
    fn route(&self, key: &str) -> RouteResult<'_, V> {
        let mut node = &self.root;
        for c in key.chars() {
            match node.children.get(&c) {
                Some(child) => node = child,
                None => return RouteResult::NotFound,
            }
        }
        match &node.value {
            Some(v) => RouteResult::Matched(v),
            None => RouteResult::NotFound,
        }
    }
}
