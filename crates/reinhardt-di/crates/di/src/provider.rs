//! Dependency providers

use crate::DiResult;
use std::any::Any;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

pub type ProviderFn = Arc<
    dyn Fn() -> Pin<Box<dyn Future<Output = DiResult<Box<dyn Any + Send + Sync>>> + Send>>
        + Send
        + Sync,
>;

pub trait Provider: Send + Sync {
    fn provide(&self)
        -> Pin<Box<dyn Future<Output = DiResult<Box<dyn Any + Send + Sync>>> + Send>>;
}

impl<F, Fut, T> Provider for F
where
    F: Fn() -> Fut + Send + Sync,
    Fut: Future<Output = DiResult<T>> + Send + 'static,
    T: Any + Send + Sync + 'static,
{
    fn provide(
        &self,
    ) -> Pin<Box<dyn Future<Output = DiResult<Box<dyn Any + Send + Sync>>> + Send>> {
        let fut = self();
        Box::pin(async move {
            let result = fut.await?;
            Ok(Box::new(result) as Box<dyn Any + Send + Sync>)
        })
    }
}
