use reinhardt_pages::server_fn::{ServerFnSet, ServerFnSetRegistration, server_fnset};

#[server_fnset(name = "items", unknown = true)]
fn items() -> impl ServerFnSetRegistration { ServerFnSet::new().named("items") }

fn main() {}
