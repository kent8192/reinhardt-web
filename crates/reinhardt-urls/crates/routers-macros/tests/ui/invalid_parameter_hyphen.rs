use reinhardt_routers_macros::path;

fn main() {
	// Error: parameter names cannot contain hyphens
	let _path = path!("/users/{user-id}/");
}
