//! Missing `name` attribute should fail

use reinhardt_macros::admin;

struct User;

#[admin(model, for = User)]
pub struct UserAdmin;

fn main() {}
