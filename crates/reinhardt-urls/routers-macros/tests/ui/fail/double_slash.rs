use reinhardt_routers_macros::path;

fn main() {
	// Error: double slash '//' not allowed
	let _path = path!("/users//posts/");
}
