//! Missing `for` attribute should fail

use reinhardt_macros::admin;

#[admin(model, name = "User")]
pub struct UserAdmin;

fn main() {}
