//! page! macro with boolean attributes using function calls

use reinhardt_pages::page;

fn is_button_disabled() -> bool {
	true
}

fn calculate_checked(value: i32) -> bool {
	value > 0
}

fn main() {
	let _valid = __reinhardt_placeholder__!(/*0*/);
}
