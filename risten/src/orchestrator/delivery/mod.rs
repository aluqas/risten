pub(crate) mod traits;

pub(crate) mod sequential;

// Expose traits
pub use sequential::SequentialDelivery;
pub use traits::DeliveryStrategy;
