//! # Domain Gateway Layer (Listener)
//!
//! A Listener is the entry point for a specific Event Domain.
//! It is responsible for interpreting raw events, transforming them into
//! domain-specific contexts, and deciding how to process them (usually by
//! delegating to a [`Router`]).
//!
//! # Layer Position
//!
//! This is **Layer 2 (Domain Gateway)** in the Risten architecture.
//! It sits between the raw event source and the dispatch logic.
//!
//! # Responsibilities
//!
//! 1. **Interpretation**: Parse raw messages into strongly-typed domain events.
//! 2. **Gatekeeping**: Filter invalid or irrelevant events early.
//! 3. **Context**: enrich events with necessary context (DB connections, User info).
//! 4. **Delegation**: Hand off the prepared event to a [`Router`] for execution.
//!
//! [`Router`]: crate::Router

use crate::{error::BoxError, handler::Handler, message::Message};
use std::{future::Future, pin::Pin};

/// A domain gateway that interprets events.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a `Listener` for `{In}`",
    label = "missing `Listener` implementation",
    note = "Listeners must implement the `listen` method to process `{In}`."
)]
pub trait Listener<In: Message>: Send + Sync + 'static {
    type Output: Message;

    fn listen(
        &self,
        event: &In,
    ) -> impl Future<Output = Result<Option<Self::Output>, BoxError>> + Send;

    /// Chains this listener with another listener.
    fn and_then<Next>(self, next: Next) -> Chain<Self, Next>
    where
        Self: Sized,
        Next: Listener<Self::Output>,
    {
        Chain {
            first: self,
            second: next,
        }
    }

    /// Filters the output of this listener.
    fn filter<F>(self, predicate: F) -> Filter<Self, F>
    where
        Self: Sized,
        F: Fn(&Self::Output) -> bool + Send + Sync + 'static,
    {
        Filter {
            listener: self,
            predicate,
        }
    }

    /// Transforms the output of this listener (sync).
    fn map<F, Out>(self, mapper: F) -> Map<Self, F>
    where
        Self: Sized,
        Out: Message,
        F: Fn(Self::Output) -> Out + Send + Sync + 'static,
    {
        Map {
            listener: self,
            mapper,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Transforms the output of this listener (async).
    fn then<F, Out, Fut>(self, mapper: F) -> Then<Self, F>
    where
        Self: Sized,
        Out: Message,
        F: Fn(Self::Output) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Out> + Send,
    {
        Then {
            listener: self,
            mapper,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Filters and maps in one step.
    fn filter_map<F, Out>(self, mapper: F) -> FilterMap<Self, F>
    where
        Self: Sized,
        Out: Message,
        F: Fn(Self::Output) -> Option<Out> + Send + Sync + 'static,
    {
        FilterMap {
            listener: self,
            mapper,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Connects to a handler.
    fn handler<H>(self, handler: H) -> Pipeline<Self, H>
    where
        Self: Sized,
        H: Handler<Self::Output>,
    {
        Pipeline {
            listener: self,
            handler,
        }
    }

    /// Catches errors.
    fn catch<F>(self, handler: F) -> Catch<Self, F>
    where
        Self: Sized,
        F: Fn(BoxError) -> Option<Self::Output> + Send + Sync + 'static,
    {
        Catch::new(self, handler)
    }

    /// Boxes the listener.
    fn boxed(self) -> BoxListener<In, Self::Output>
    where
        Self: Sized,
        In: Sync,
    {
        BoxListener::new(self)
    }
}

pub struct Chain<A, B> {
    pub(crate) first: A,
    pub(crate) second: B,
}

impl<A, B, In> Listener<In> for Chain<A, B>
where
    In: Message + Sync,
    A: Listener<In>,
    A::Output: Sync,
    B: Listener<A::Output>,
{
    type Output = B::Output;

    async fn listen(&self, event: &In) -> Result<Option<Self::Output>, BoxError> {
        let Some(intermediate) = self.first.listen(event).await? else {
            return Ok(None);
        };
        self.second.listen(&intermediate).await
    }
}

pub struct Filter<L, F> {
    listener: L,
    predicate: F,
}

impl<L, F, In> Listener<In> for Filter<L, F>
where
    In: Message + Sync,
    L: Listener<In>,
    L::Output: Clone + Sync,
    F: Fn(&L::Output) -> bool + Send + Sync + 'static,
{
    type Output = L::Output;

    async fn listen(&self, event: &In) -> Result<Option<Self::Output>, BoxError> {
        let Some(output) = self.listener.listen(event).await? else {
            return Ok(None);
        };
        if (self.predicate)(&output) {
            Ok(Some(output))
        } else {
            Ok(None)
        }
    }
}

pub struct Map<L, F, Out = ()> {
    listener: L,
    mapper: F,
    _phantom: std::marker::PhantomData<Out>,
}

impl<L, F, In, Out> Listener<In> for Map<L, F, Out>
where
    In: Message + Sync,
    L: Listener<In>,
    L::Output: Sync,
    Out: Message,
    F: Fn(L::Output) -> Out + Send + Sync + 'static,
{
    type Output = Out;

    async fn listen(&self, event: &In) -> Result<Option<Self::Output>, BoxError> {
        let Some(output) = self.listener.listen(event).await? else {
            return Ok(None);
        };
        Ok(Some((self.mapper)(output)))
    }
}

pub struct Then<L, F, Out = ()> {
    listener: L,
    mapper: F,
    _phantom: std::marker::PhantomData<Out>,
}

impl<L, F, In, Out, Fut> Listener<In> for Then<L, F, Out>
where
    In: Message + Sync,
    L: Listener<In>,
    L::Output: Sync,
    Out: Message,
    F: Fn(L::Output) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Out> + Send,
{
    type Output = Out;

    async fn listen(&self, event: &In) -> Result<Option<Self::Output>, BoxError> {
        let Some(output) = self.listener.listen(event).await? else {
            return Ok(None);
        };
        Ok(Some((self.mapper)(output).await))
    }
}

pub struct FilterMap<L, F, Out = ()> {
    listener: L,
    mapper: F,
    _phantom: std::marker::PhantomData<Out>,
}

impl<L, F, In, Out> Listener<In> for FilterMap<L, F, Out>
where
    In: Message + Sync,
    L: Listener<In>,
    L::Output: Sync,
    Out: Message,
    F: Fn(L::Output) -> Option<Out> + Send + Sync + 'static,
{
    type Output = Out;

    async fn listen(&self, event: &In) -> Result<Option<Self::Output>, BoxError> {
        let Some(output) = self.listener.listen(event).await? else {
            return Ok(None);
        };
        Ok((self.mapper)(output))
    }
}

pub struct Pipeline<L, H> {
    pub listener: L,
    pub handler: H,
}

use crate::{
    handler::HandlerResult,
    hook::{Hook, HookResult},
    response::IntoResponse,
};

impl<L, H, In> Hook<In> for Pipeline<L, H>
where
    In: Message + Sync,
    L: Listener<In>,
    H: Handler<L::Output>,
    L::Output: Send + Sync,
    H::Output: HandlerResult + IntoResponse,
{
    async fn on_event(
        &self,
        event: &In,
    ) -> Result<HookResult, Box<dyn std::error::Error + Send + Sync>> {
        match self.listener.listen(event).await {
            Ok(Some(out)) => {
                let result = self.handler.call(out).await;
                result.into_response()
            }
            Ok(None) => Ok(HookResult::Next),
            Err(e) => Err(e),
        }
    }
}

pub struct BoxListener<In, Out> {
    inner: Box<dyn DynListener<In, Output = Out>>,
}

impl<In, Out> BoxListener<In, Out>
where
    In: Message,
    Out: Message,
{
    pub fn new<L>(listener: L) -> Self
    where
        L: Listener<In, Output = Out>,
        In: Sync,
    {
        Self {
            inner: Box::new(listener),
        }
    }
}

impl<In, Out> Listener<In> for BoxListener<In, Out>
where
    In: Message + Sync,
    Out: Message,
{
    type Output = Out;

    async fn listen(&self, event: &In) -> Result<Option<Self::Output>, BoxError> {
        self.inner.listen_dyn(event).await
    }
}

pub trait DynListener<In>: Send + Sync + 'static {
    type Output: Message;
    fn listen_dyn<'a>(
        &'a self,
        event: &'a In,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Self::Output>, BoxError>> + Send + 'a>>;
}

impl<L, In> DynListener<In> for L
where
    L: Listener<In>,
    In: Message + Sync,
{
    type Output = L::Output;

    fn listen_dyn<'a>(
        &'a self,
        event: &'a In,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Self::Output>, BoxError>> + Send + 'a>> {
        Box::pin(self.listen(event))
    }
}

pub struct Catch<L, F> {
    listener: L,
    handler: F,
}

impl<L, F> Catch<L, F> {
    pub fn new(listener: L, handler: F) -> Self {
        Self { listener, handler }
    }
}

impl<L, F, In> Listener<In> for Catch<L, F>
where
    In: Message + Sync,
    L: Listener<In>,
    L::Output: Sync,
    F: Fn(BoxError) -> Option<L::Output> + Send + Sync + 'static,
{
    type Output = L::Output;

    async fn listen(&self, event: &In) -> Result<Option<Self::Output>, BoxError> {
        match self.listener.listen(event).await {
            Ok(result) => Ok(result),
            Err(e) => Ok((self.handler)(e)),
        }
    }
}
