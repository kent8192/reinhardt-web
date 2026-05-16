//! Regression test for issue #4445.
//!
//! Verifies that the `#[injectable]` macro emits an `async_trait` attribute
//! path that is reachable without the consumer depending on the `async-trait`
//! crate directly. The macro routes through `reinhardt_core::async_trait`
//! (a re-export), so this fixture deliberately only `use`s symbols from
//! `reinhardt_di_macros` and `reinhardt_di`, never `async_trait` itself.

use reinhardt_di::{Depends, Injectable};
use reinhardt_di_macros::injectable;

// Path A: no #[inject] fields — exercises the Default-based `Injectable` impl.
#[injectable]
#[derive(Clone, Default)]
struct ConfigService {
	#[no_inject(default = Default)]
	_host: String,
}

// A minimal injectable target so that `Depends<ServiceA>` resolves to a real
// `Injectable`-implementing type.
#[injectable]
#[derive(Clone, Default)]
struct ServiceA;

// Path B: with an #[inject] field — exercises the field-injection `Injectable`
// impl. Both emission paths must reach `async_trait` through the
// reinhardt-core re-export rather than a bare `async_trait::async_trait`.
#[injectable]
struct ServiceB {
	#[inject]
	_a: Depends<ServiceA>,
}

fn main() {
	// `Injectable` must be implemented for both structs. Use the trait at the
	// type level so the trait bound is checked without requiring a runtime
	// `InjectionContext`.
	fn _assert_injectable<T: Injectable>() {}
	_assert_injectable::<ConfigService>();
	_assert_injectable::<ServiceA>();
	_assert_injectable::<ServiceB>();
}
