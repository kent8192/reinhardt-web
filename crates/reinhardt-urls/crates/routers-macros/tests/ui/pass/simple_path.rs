use reinhardt_routers_macros::path;

fn main() {
	let _ = path!("/users/");
	let _ = path!("/items/");
	let _ = path!("/api/v1/users/");
}
