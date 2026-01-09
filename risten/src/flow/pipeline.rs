pub use crate::flow::listener::Pipeline;
use crate::{
    core::message::Message,
    flow::{
        handler::{Handler, HandlerResult, IntoHookOutcome},
        hook::{Hook, HookResult},
        listener::Listener,
    },
};
// #[async_trait] - Removed for native async trait
impl<L, H, In> Hook<In> for Pipeline<L, H>
where
    In: Message + Sync,
    L: Listener<In>,
    H: Handler<L::Output>,
    L::Output: Send + Sync,
    H::Output: HandlerResult + IntoHookOutcome,
{
    async fn on_event(
        &self,
        event: &In,
    ) -> Result<HookResult, Box<dyn std::error::Error + Send + Sync>> {
        // Phase 1: Listener (Sync, Borrow)
        // If the listener returns None, we just ignore this event (Next).
        // If it returns Some(out), we proceed to Phase 2.
        if let Some(out) = self.listener.listen(event) {
            // Phase 2: Handler (Async, Own)
            // We pass ownership of the extracted 'out' to the handler.
            let result = self.handler.call(out).await;

            // Convert the handler's output to a hook outcome
            result.into_response()
        } else {
            Ok(HookResult::Next)
        }
    }
}
