//! page! macro with props (closure parameters)

use reinhardt_pages::page;

fn main() {
	// Single prop
	let _greeting = page!(|name: String| {
		span {
			name
		}
	});

	// Multiple props
	let _user_card = page!(|name: String, age: i32| {
		div {
			class: "user-card",
			span {
				name
			}
			span {
				age.to_string()
			}
		}
	});

	// Props with trailing comma
	let _trailing = page!(|x: i32| {
		div {
			x.to_string()
		}
	});
}
