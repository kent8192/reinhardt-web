use reinhardt_testkit_macros::with_di_overrides;

struct HttpClient;

fn main() {
	let _result = with_di_overrides! {
		transient HttpClient => |_ctx| async {
			Ok::<HttpClient, ::reinhardt_di::DiError>(HttpClient)
		},
	};
}
