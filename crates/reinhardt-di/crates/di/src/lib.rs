//! # Dependency Injection Core
//!
//! FastAPI-inspired dependency injection system.

pub mod context;
pub mod depends;
pub mod injectable;
pub mod provider;
pub mod scope;

use thiserror::Error;

pub use context::{InjectionContext, RequestContext};
pub use depends::{Depends, DependsBuilder};
pub use injectable::Injectable;
pub use provider::{Provider, ProviderFn};
pub use scope::{RequestScope, Scope, SingletonScope};

#[derive(Debug, Error)]
pub enum DiError {
	#[error("Dependency not found: {0}")]
	NotFound(String),

	#[error("Circular dependency detected: {0}")]
	CircularDependency(String),

	#[error("Provider error: {0}")]
	ProviderError(String),

	#[error("Type mismatch: expected {expected}, got {actual}")]
	TypeMismatch { expected: String, actual: String },

	#[error("Scope error: {0}")]
	ScopeError(String),
}

pub type DiResult<T> = std::result::Result<T, DiError>;
