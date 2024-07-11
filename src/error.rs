use std::error::Error;
use thiserror::Error;
use winit::error::EventLoopError;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("internal error: {0}")]
    Internal(#[from] Box<dyn std::error::Error + Send + Sync>),
}

pub type AppResult<T> = Result<T, AppError>;

macro_rules! impl_internal_errors {
    ( $( $type:ty ),* $(,)? ) => {
        $(
        impl From<$type> for AppError {
            fn from(err: $type) -> Self {
                AppError::Internal(Box::new(err))
            }
        }
        )*
    };
}

impl_internal_errors!(EventLoopError, std::io::Error);
