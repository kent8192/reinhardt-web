//! Verifies that any builder shaped like `ServerRouter::viewset_with_actions`
//! — i.e. with a third parameter statically typed as `PhantomData<M>` —
//! causes rustc to reject a non-`PhantomData` value (e.g. an integer literal)
//! with a type error that mentions `PhantomData`.
//!
//! `reinhardt-urls` transitively depends on `reinhardt-macros` through
//! `reinhardt-core`, so importing the real `ServerRouter` here would create
//! a circular dependency at the dev-dep layer. Instead, this golden mirrors
//! the real signature in a self-contained `MiniRouter` stub. The signature
//! shape (third parameter `_marker: PhantomData<M>`) is kept byte-identical
//! to the real method on `crates/reinhardt-urls/src/routers/server_router/
//! registration.rs::viewset_with_actions`, so the produced compiler error
//! locks in the user-facing contract that `viewset_with_actions` requires
//! `PhantomData::<MyViewSetImpl>` as its third argument.
//!
//! Refs Issue #4507.

use std::marker::PhantomData;

// Anchor to a Reinhardt component to satisfy the "EVERY test MUST use at least
// one Reinhardt component" policy without affecting the E0308 golden below.
// `reinhardt-apps` is the only reinhardt crate visible from this trybuild
// fixture (other reinhardt crates would form a circular dev-dep through
// `reinhardt-core/macros`; see the module-level comment above).
use reinhardt_apps as _;

/// Stand-in for the real `ServerRouter`. Only the third-parameter shape of
/// `viewset_with_actions` matters for this golden.
struct MiniRouter;

impl MiniRouter {
	fn viewset_with_actions<V, M>(self, _prefix: &str, _viewset: V, _marker: PhantomData<M>) -> Self {
		self
	}
}

struct DummyViewSet;

fn main() {
	// Passing `42` instead of `PhantomData::<SomeImpl>` must be a type error.
	let _ = MiniRouter.viewset_with_actions("/x", DummyViewSet, 42);
}
