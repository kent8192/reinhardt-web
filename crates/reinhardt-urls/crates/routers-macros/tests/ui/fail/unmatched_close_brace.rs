use reinhardt_routers_macros::path;

fn main() {
	// Error: unmatched closing brace '}'
	let _path = path!("/users/id}/");
}
