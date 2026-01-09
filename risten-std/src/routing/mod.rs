//! Static routing implementations.
//!
//! These routers are compile-time fixed or read-only at runtime.

pub mod trie;

pub use trie::TrieRouter;
