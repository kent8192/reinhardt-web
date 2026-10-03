// Rejected macro items leave their supporting imports unused.
#![allow(unused_imports)]

use reinhardt_pages::server_fn::{ServerFnSet, ServerFnSetRegistration, server_fnset};

#[server_fnset(name = "")]
fn empty() -> impl ServerFnSetRegistration { ServerFnSet::new().named("unused") }

fn main() {}
