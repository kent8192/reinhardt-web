use reinhardt_routers_macros::path;

fn main() {
	// Error: spaces not allowed in path
	let _path = path!("/user profiles/");
}
