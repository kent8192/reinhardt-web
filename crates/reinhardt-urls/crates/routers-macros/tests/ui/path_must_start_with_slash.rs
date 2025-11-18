use reinhardt_routers_macros::path;

fn main() {
	// Error: path must start with '/'
	let _path = path!("users/");
}
