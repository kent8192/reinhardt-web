//! Unknown attribute should fail

use reinhardt_macros::admin;

struct User;

#[admin(model, for = User, name = "User", unknown_attr = "value")]
pub struct UserAdmin;

fn main() {}
