//! # Routing Implementations
//!
//! This module provides various router implementations:
//!
//! - **Static routing**: Compile-time fixed hook chains via HList.
//! - **Dispatch routing**: Inventory-based automatic handler collection.
//!
//! # Choosing a Router
//!
//! | Router | Use Case | Performance |
//! |--------|----------|-------------|
//! | `StaticRouter` | Known handlers at compile time | Zero-cost, fully inlined |
//! | `DispatchRouter` | Dynamic handler discovery | Small runtime overhead |

#[cfg(feature = "inventory")]
pub mod dispatch;

#[cfg(feature = "inventory")]
pub use dispatch::{
    ConfigurableDispatchRouter, DispatchError, DispatchMode, DispatchRouter, ErasedHandler,
    ErasedHandlerWrapper, HandlerRegistration, SequentialDispatchRouter,
};
