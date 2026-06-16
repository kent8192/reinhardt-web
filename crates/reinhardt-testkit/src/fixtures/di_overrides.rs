//! Fixtures for overriding `#[injectable]` provider-managed dependencies in tests.
//!
//! See `docs/superpowers/specs/2026-05-11-di-mock-fixtures-design.md` for the
//! design rationale.
//!
//! # Example
//!
//! ```rust,no_run
//! use reinhardt_testkit::with_di_overrides;
//! use rstest::*;
//!
//! #[derive(Clone, Debug, PartialEq)]
//! struct Config { url: String }
//!
//! #[rstest]
//! #[tokio::test]
//! async fn test_my_flow() {
//!     let (ctx, _di) = with_di_overrides! {
//!         singleton Config { url: "test://db".to_string() },
//!     };
//!     let cfg: std::sync::Arc<Config> = ctx.get_singleton().unwrap();
//!     assert_eq!(cfg.url, "test://db");
//! }
//! ```

use std::any::Any;
use std::future::Future;
use std::sync::Arc;

use reinhardt_di::{
	DependencyRegistry, DependencyScope, DiResult, InjectionContext, OverrideGuard, SingletonScope,
};

/// Holds all `OverrideGuard`s installed during a test. Drop reverts them.
pub struct DiOverrides {
	_guards: Vec<OverrideGuard>,
}

/// Seed closure that pre-populates a value into a freshly built
/// `InjectionContext`'s request scope. Boxed so heterogeneous closures (one per
/// `request` call) can be collected in a `Vec`.
type RequestSeedFn = Box<dyn FnOnce(&InjectionContext) + Send + 'static>;

/// Builder passed to the setup closure of
/// [`injection_context_with_di_overrides`].
pub struct DiOverrideBuilder<'a> {
	registry: Arc<DependencyRegistry>,
	scope: &'a SingletonScope,
	guards: Vec<OverrideGuard>,
	// Request-scoped seed closures applied to the constructed `InjectionContext`
	// after build, so the values land in `RequestScope` (not `SingletonScope`)
	// and surface through `ctx.get_request::<T>()`. Held separately from
	// `guards` because seeds run once at context-build time and are then
	// dropped, while guards must live for the entire test scope to keep the
	// per-context registry overrides in effect.
	request_seeds: Vec<RequestSeedFn>,
}

impl<'a> DiOverrideBuilder<'a> {
	fn new(registry: Arc<DependencyRegistry>, scope: &'a SingletonScope) -> Self {
		Self {
			registry,
			scope,
			guards: Vec::new(),
			request_seeds: Vec::new(),
		}
	}

	/// Override a `Singleton`-scoped type by pre-seeding the singleton scope.
	///
	/// No registry mutation — the value is placed directly into the
	/// per-context singleton scope.
	pub fn singleton<T: Any + Send + Sync + 'static>(&mut self, value: T) {
		self.scope.set(value);
	}

	/// Override a `Request`-scoped type by pre-seeding the request scope of
	/// the constructed context.
	///
	/// The value is queued here and applied via
	/// [`InjectionContext::set_request`] after the surrounding
	/// [`injection_context_with_di_overrides`] builds the context, so it
	/// surfaces through `ctx.get_request::<T>()` and the
	/// `DependencyScope::Request` cache lookup in `ctx.resolve::<T>()`.
	pub fn request_value<T: Any + Send + Sync + 'static>(&mut self, value: T) {
		self.request_seeds.push(Box::new(move |ctx| {
			ctx.set_request::<T>(value);
		}));
	}

	/// Override an arbitrary factory.
	///
	/// Installs the factory into the per-context registry via
	/// [`DependencyRegistry::register_override`]. Because each test context
	/// receives its own isolated registry, `#[serial(di_registry)]` is NOT
	/// required.
	pub fn factory<T, F, Fut>(&mut self, scope: DependencyScope, factory: F)
	where
		T: Any + Send + Sync + 'static,
		F: Fn(Arc<InjectionContext>) -> Fut + Send + Sync + 'static,
		Fut: Future<Output = DiResult<T>> + Send + 'static,
	{
		let guard = self.registry.register_override::<T, _, _>(scope, factory);
		self.guards.push(guard);
	}

	fn into_overrides(self) -> DiOverrides {
		DiOverrides {
			_guards: self.guards,
		}
	}
}

/// Build an `InjectionContext` with the overrides supplied by the setup
/// closure. Returns the context plus a `DiOverrides` token that must be kept
/// alive for the duration of the test (drop reverts all overrides).
///
/// Each invocation creates an isolated per-context registry. Factory
/// overrides installed via [`DiOverrideBuilder::factory`] land in that
/// registry, not in the global one, so tests can run in parallel without
/// `#[serial(di_registry)]`.
pub async fn injection_context_with_di_overrides<F>(setup: F) -> (InjectionContext, DiOverrides)
where
	F: FnOnce(&SingletonScope, &mut DiOverrideBuilder<'_>),
{
	let scope = Arc::new(SingletonScope::new());
	let per_context_registry = Arc::new(DependencyRegistry::new());
	let mut builder = DiOverrideBuilder::new(Arc::clone(&per_context_registry), &scope);
	setup(&scope, &mut builder);
	// Extract the request-scope seed closures before consuming the builder.
	let request_seeds = std::mem::take(&mut builder.request_seeds);
	let overrides = builder.into_overrides();
	let ctx = InjectionContext::builder(scope)
		.with_registry(per_context_registry)
		.build();
	for seed in request_seeds {
		seed(&ctx);
	}
	(ctx, overrides)
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::*;

	#[derive(Clone, Debug, PartialEq)]
	struct Cfg {
		key: &'static str,
	}

	#[rstest]
	#[tokio::test]
	async fn singleton_override_is_visible_via_get_singleton() {
		// Arrange
		let (ctx, _di) = injection_context_with_di_overrides(|_scope, builder| {
			builder.singleton(Cfg { key: "test" });
		})
		.await;

		// Act
		let value: Option<Arc<Cfg>> = ctx.get_singleton();

		// Assert
		assert_eq!(value.unwrap().key, "test");
	}

	#[derive(Clone, Debug, PartialEq)]
	struct Counter(u32);

	#[rstest]
	#[tokio::test]
	async fn factory_override_returns_mock_value() {
		// Arrange
		let (ctx, _di) = injection_context_with_di_overrides(|_scope, builder| {
			builder.factory::<Counter, _, _>(DependencyScope::Transient, |_ctx| async {
				Ok::<_, reinhardt_di::DiError>(Counter(7))
			});
		})
		.await;

		// Act
		let value: Arc<Counter> = ctx.resolve::<Counter>().await.unwrap();

		// Assert
		assert_eq!(*value, Counter(7));
	}

	#[derive(Clone, Debug, PartialEq)]
	struct RequestCfg {
		token: &'static str,
	}

	#[rstest]
	#[tokio::test]
	async fn request_value_lands_in_request_scope_not_singleton() {
		// Arrange
		let (ctx, _di) = injection_context_with_di_overrides(|_scope, builder| {
			builder.request_value(RequestCfg { token: "req-only" });
		})
		.await;

		// Act
		let from_request: Option<Arc<RequestCfg>> = ctx.get_request();
		let from_singleton: Option<Arc<RequestCfg>> = ctx.get_singleton();

		// Assert -- value is visible from the request scope, NOT from the
		// singleton scope. This is the round-trip the macro consumer expects
		// when writing `request <Type> <value>`.
		assert_eq!(from_request.unwrap().token, "req-only");
		assert!(from_singleton.is_none());
	}

	#[rstest]
	#[tokio::test]
	async fn factory_override_reverts_after_di_overrides_drop() {
		// Arrange -- install a production factory once
		let registry = reinhardt_di::global_registry().clone();
		#[derive(Clone, Debug, PartialEq)]
		struct DropProbe(u32);

		if !registry.is_registered::<DropProbe>() {
			registry.register_async::<DropProbe, _, _>(DependencyScope::Transient, |_ctx| async {
				Ok(DropProbe(1))
			});
		}

		// Act -- override, then drop the overrides token
		{
			let (ctx, di) = injection_context_with_di_overrides(|_scope, builder| {
				builder.factory::<DropProbe, _, _>(DependencyScope::Transient, |_ctx| async {
					Ok::<_, reinhardt_di::DiError>(DropProbe(99))
				});
			})
			.await;
			let v: Arc<DropProbe> = ctx.resolve::<DropProbe>().await.unwrap();
			assert_eq!(*v, DropProbe(99));
			drop(di);
		}

		// Assert -- production factory is back
		let ctx = InjectionContext::builder(Arc::new(SingletonScope::new())).build();
		let v: Arc<DropProbe> = ctx.resolve::<DropProbe>().await.unwrap();
		assert_eq!(*v, DropProbe(1));
	}
}
