use reinhardt_routers_macros::path;

fn main() {
	// Error: unmatched opening brace '{'
	let _path = path!("/users/{id/");
}
