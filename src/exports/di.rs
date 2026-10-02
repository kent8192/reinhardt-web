//! Dependency injection and parameter extraction re-exports.

#[cfg(feature = "di")]
pub use reinhardt_di::scope::{RequestScope, Scope, SingletonScope};
#[cfg(feature = "di")]
pub use reinhardt_di::{
	Depends, DependsBuilder, DiError, DiResult, Injectable, InjectableKey, InjectionContext,
	InjectionContextBuilder, InjectionMetadata, KeyedDepends, KeyedDependsBuilder,
	KeyedFactoryOutput, RequestContext, SelfKey, injectable, injectable_key,
};
// Keep the deprecated alias available through the facade until the alias itself
// is removed from reinhardt-di.
#[allow(deprecated)]
#[cfg(feature = "di")]
pub use reinhardt_di::FactoryOutput;

#[cfg(any(feature = "minimal", feature = "standard", feature = "di"))]
pub use reinhardt_di::params::{
	Body, Cookie, CookieName, CookieNamed, CookieStruct, Header, Json, Path, Query,
};
