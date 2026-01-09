use super::traits::DeliveryStrategy;
use crate::{
    core::{error::DispatchError, message::Message},
    flow::hook::{DynHook, HookResult},
};

/// A sequential delivery strategy.
///
/// Executes hooks one by one. Stops if a hook returns `HookResult::Stop` or an error.
#[derive(Debug, Default, Clone, Copy)]
pub struct SequentialDelivery;

impl DeliveryStrategy for SequentialDelivery {
    async fn deliver<'a, E, I>(&self, event: E, hooks: I) -> Result<(), DispatchError>
    where
        E: Message + Sync + 'a,
        I: Iterator<Item = &'a dyn DynHook<E>> + Send + 'a,
    {
        for hook in hooks {
            match hook.on_event_dyn(&event).await {
                Ok(HookResult::Next) => continue,
                Ok(HookResult::Stop) => return Ok(()),
                Err(e) => {
                    return Err(DispatchError::ListenerError(e));
                }
            }
        }
        Ok(())
    }
}
