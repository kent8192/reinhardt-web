use reinhardt_routers_macros::path;

fn main() {
	// Error: special characters like @ not allowed in path
	let _path = path!("/users/@admin/");
}
