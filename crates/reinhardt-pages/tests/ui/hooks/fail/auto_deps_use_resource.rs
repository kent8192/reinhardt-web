//! Compile-fail: resources require an explicit `deps![...]` list because
//! fetchers run asynchronously and cannot use automatic closure tracking.

use reinhardt_pages::deps_auto;
use reinhardt_pages::reactive::use_resource;

fn main() {
	let _resource = use_resource(
		move || async move { Ok::<i32, String>(1) },
		deps_auto!(),
	);
}
