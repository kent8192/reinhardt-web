//! Typed, ordered registration of server function marker sets.

pub mod metadata;

use super::ServerFnMetadata;
#[cfg(native)]
use super::{ServerFnRegistration, ServerFnRouterExt};
pub use metadata::{ServerFnSetActionMetadata, ServerFnSetMetadata};
#[cfg(native)]
use reinhardt_urls::routers::ServerRouter;

/// Entry point for constructing a typed server function set.
pub struct ServerFnSet;

impl ServerFnSet {
	/// Creates an empty typed server function chain.
	// The public builder intentionally starts with the concrete nil chain type.
	#[allow(clippy::new_ret_no_self)]
	pub const fn new() -> ServerFnSetNil {
		ServerFnSetNil
	}
}

/// Empty terminator for a typed server function set chain.
pub struct ServerFnSetNil;

/// One server function marker followed by the rest of a typed set chain.
pub struct ServerFnSetCons<H, T> {
	// WASM retains the typed chain for shared declarations but never performs native registration.
	#[cfg_attr(wasm, allow(dead_code))]
	pub(crate) head: H,
	pub(crate) tail: T,
}

/// A typed server function set with an application-facing name.
pub struct NamedServerFnSet<S> {
	name: &'static str,
	set: S,
}

mod sealed {
	pub trait Sealed {}
}

/// Sealed implementation trait for server function set chains.
#[doc(hidden)]
pub trait ServerFnSetChain: sealed::Sealed {
	#[doc(hidden)]
	fn append_metadata(&self, actions: &mut Vec<ServerFnSetActionMetadata>);
}

impl sealed::Sealed for ServerFnSetNil {}

impl ServerFnSetChain for ServerFnSetNil {
	fn append_metadata(&self, _actions: &mut Vec<ServerFnSetActionMetadata>) {}
}

impl<H, T> sealed::Sealed for ServerFnSetCons<H, T>
where
	H: ServerFnMetadata,
	T: ServerFnSetChain,
{
}

impl<H, T> ServerFnSetChain for ServerFnSetCons<H, T>
where
	H: ServerFnMetadata,
	T: ServerFnSetChain,
{
	fn append_metadata(&self, actions: &mut Vec<ServerFnSetActionMetadata>) {
		self.tail.append_metadata(actions);
		actions.push(ServerFnSetActionMetadata {
			name: H::NAME,
			path: H::PATH,
			codec: H::CODEC,
			injected_params: H::INJECTED_PARAMS,
			detail: H::DETAIL,
			transactional: H::TRANSACTIONAL,
		});
	}
}

/// Builder methods available on every typed server function set chain.
pub trait ServerFnSetChainExt: ServerFnSetChain + Sized {
	/// Appends a generated server function marker to this set.
	fn server_fn<H: ServerFnMetadata>(self, head: H) -> ServerFnSetCons<H, Self> {
		ServerFnSetCons { head, tail: self }
	}

	/// Assigns an application-facing name to this set.
	fn named(self, name: &'static str) -> NamedServerFnSet<Self> {
		NamedServerFnSet { name, set: self }
	}
}

impl<S: ServerFnSetChain> ServerFnSetChainExt for S {}

/// Metadata and native router registration behavior for a named set.
pub trait ServerFnSetRegistration: Sized {
	/// Returns owned metadata in builder order.
	fn metadata(&self) -> ServerFnSetMetadata;

	/// Registers every member with the native server router.
	#[cfg(native)]
	fn register(self, router: ServerRouter) -> ServerRouter;
}

/// Explicit action manifest generated for a model server function set.
pub trait ServerFnSetActions<R> {
	/// Concrete typed marker chain registered for the resource.
	type Registration: ServerFnSetChain;

	/// Build the complete ordered action registration.
	fn registration() -> Self::Registration;
}

#[cfg(not(native))]
impl<S: ServerFnSetChain> ServerFnSetRegistration for NamedServerFnSet<S> {
	fn metadata(&self) -> ServerFnSetMetadata {
		let mut actions = Vec::new();
		self.set.append_metadata(&mut actions);
		ServerFnSetMetadata {
			name: self.name,
			actions,
		}
	}
}

#[cfg(native)]
trait NativeServerFnSetChain {
	fn register(self, router: ServerRouter) -> ServerRouter;
}

#[cfg(native)]
impl NativeServerFnSetChain for ServerFnSetNil {
	fn register(self, router: ServerRouter) -> ServerRouter {
		router
	}
}

#[cfg(native)]
impl<H, T> NativeServerFnSetChain for ServerFnSetCons<H, T>
where
	H: ServerFnRegistration + 'static,
	T: NativeServerFnSetChain,
{
	fn register(self, router: ServerRouter) -> ServerRouter {
		let router = self.tail.register(router);
		router.server_fn(self.head)
	}
}

#[cfg(native)]
impl<S> ServerFnSetRegistration for NamedServerFnSet<S>
where
	S: ServerFnSetChain + NativeServerFnSetChain,
{
	fn metadata(&self) -> ServerFnSetMetadata {
		let mut actions = Vec::new();
		self.set.append_metadata(&mut actions);
		ServerFnSetMetadata {
			name: self.name,
			actions,
		}
	}

	fn register(self, router: ServerRouter) -> ServerRouter {
		self.set.register(router)
	}
}
