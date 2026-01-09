//! Pre-configured combinations of strategies for common use cases.

#[cfg(feature = "phf")]
use crate::source::router::PhfRouter;

#[cfg(feature = "phf")]
use crate::orchestrator::traits::DynDispatcher;

/// A "Fast Track" router configuration.
///
/// This combines:
/// - `PhfRouter`: Compile-time routing (O(1) lookup).
/// - `StaticFanoutDispatcher`: Compile-time parallel execution of hooks.
/// - `DynDispatcher`: Object-safe dispatch to allow heterogeneous hook chains in the map.
///
/// # Usage
///
/// ```rust,ignore
/// static ROUTER: FastRouter<MyEvent> = phf_router! {
///     "/path" => &MY_STATIC_DISPATCHER,
/// };
/// ```
#[cfg(feature = "phf")]
pub type FastRouter<E> = PhfRouter<&'static (dyn DynDispatcher<E> + Sync)>;
