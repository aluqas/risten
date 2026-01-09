pub(crate) mod delivery;
pub(crate) mod dynamic;
pub(crate) mod macros;
pub(crate) mod registry;
pub(crate) mod traits;

// To be consolidated later, for now exposing to fix build
// pub(crate) mod standard; // Consolidated into dynamic
pub(crate) mod r#static;
pub(crate) mod static_fanout;

// Re-export key components

pub use dynamic::DynamicDispatcher;
pub use registry::{EnabledHandle, RegistrationMeta, Registry, RegistryBuilder};
pub use traits::{Dispatcher, HookProvider};
