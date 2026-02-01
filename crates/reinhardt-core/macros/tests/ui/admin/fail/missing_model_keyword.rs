//! Missing `model` keyword should fail

use reinhardt_macros::admin;

struct User;

#[admin(for = User, name = "User")]
pub struct UserAdmin;

fn main() {}
