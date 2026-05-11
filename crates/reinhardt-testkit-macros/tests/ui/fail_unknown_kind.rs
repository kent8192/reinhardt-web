use reinhardt_testkit_macros::with_di_overrides;

struct Cfg;

fn main() {
	let _ = with_di_overrides! {
		bogus Cfg => |_ctx| async { Ok(Cfg) },
	};
}
