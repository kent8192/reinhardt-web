//! Basic admin macro usage with required attributes

use reinhardt_macros::admin;

struct User;

#[admin(model, for = User, name = "User")]
pub struct UserAdmin;

fn main() {
	// Compile test only - verify the macro expands without errors
}
