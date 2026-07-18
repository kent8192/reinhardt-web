use reinhardt_pages::server_fn::{
	ServerFnError, ServerFnSet, ServerFnSetChainExt, ServerFnSetRegistration, server_fn,
	server_fnset,
};

#[server_fn]
async fn first() -> Result<(), ServerFnError> {
	Ok(())
}

#[server_fn]
async fn second() -> Result<(), ServerFnError> {
	Ok(())
}

#[server_fnset(name = "admin")]
pub fn admin_fns() -> impl ServerFnSetRegistration {
	ServerFnSet::new()
		.server_fn(first::marker)
		.server_fn(second::marker)
}

fn main() {
	let metadata = admin_fns().metadata();
	assert_eq!(metadata.name, "admin");
	assert!(metadata.actions.iter().all(|action| !action.detail));
	assert!(metadata.actions.iter().all(|action| !action.transactional));
}
