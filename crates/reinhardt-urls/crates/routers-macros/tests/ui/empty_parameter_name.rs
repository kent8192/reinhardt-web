use reinhardt_routers_macros::path;

fn main() {
	// Error: empty parameter name
	let _path = path!("/users/{}/");
}
