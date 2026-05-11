use reinhardt_testkit_macros::with_di_overrides;

struct HttpClient;

fn main() {
	// The macro emits `.await`, so the call site must be inside an `async`
	// block.
	let _fut = async {
		let _result = with_di_overrides! {
			transient HttpClient => |_ctx| async {
				Ok::<HttpClient, ::reinhardt_testkit::DiError>(HttpClient)
			},
		};
	};
}
