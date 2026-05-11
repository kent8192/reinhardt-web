use reinhardt_testkit_macros::with_di_overrides;

#[derive(Clone)]
struct Cfg {
	key: &'static str,
}

fn main() {
	// We only need this to compile. The runtime semantics are exercised in
	// reinhardt-testkit's fixtures::di_overrides test module.
	let _result = with_di_overrides! {
		singleton Cfg { key: "test" },
	};
}
