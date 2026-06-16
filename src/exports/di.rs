//! Dependency injection and parameter extraction re-exports.

#[cfg(feature = "di")]
pub use reinhardt_di::scope::{RequestScope, Scope, SingletonScope};
#[cfg(feature = "di")]
pub use reinhardt_di::{
	Depends, DependsBuilder, DiError, DiResult, FactoryOutput, Injectable, InjectableKey,
	InjectionContext, InjectionContextBuilder, InjectionMetadata, RequestContext,
	injectable_key,
};

#[cfg(any(feature = "minimal", feature = "standard", feature = "di"))]
pub use reinhardt_di::params::{Body, Cookie, Header, Json, Path, Query};
