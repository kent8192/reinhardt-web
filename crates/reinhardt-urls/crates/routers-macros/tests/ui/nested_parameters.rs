use reinhardt_routers_macros::path;

fn main() {
	// Error: nested parameters not allowed
	let _path = path!("/users/{{id}}/");
}
