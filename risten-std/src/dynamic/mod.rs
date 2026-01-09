//! Dynamic dispatch support.
//!
//! This module provides runtime-flexible dispatching mechanisms.
//! Use when hook composition is determined at runtime (plugins, config-driven).

pub mod registry;
pub mod routing;

pub use registry::{Registry, RegistryBuilder};
pub use routing::HashMapRouter;
