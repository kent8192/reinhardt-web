use reinhardt_pages::server_fn::server_fnset;

#[server_fnset(name = "{tenant}")]
fn admin_fns() -> impl reinhardt_pages::server_fn::ServerFnSetRegistration {
	reinhardt_pages::server_fn::ServerFnSet::new()
}

fn main() {}
