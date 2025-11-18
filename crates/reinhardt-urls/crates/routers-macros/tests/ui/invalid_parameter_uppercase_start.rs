use reinhardt_routers_macros::path;

fn main() {
	// Error: parameter names must start with lowercase letter or underscore
	let _path = path!("/users/{Id}/");
}
