use thiserror::Error;

pub type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

#[derive(Error, Debug)]
pub enum DispatchError {
    #[error(transparent)]
    ListenerError(#[from] BoxError),
}
