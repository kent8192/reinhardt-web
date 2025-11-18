use reinhardt_routers_macros::path;

fn main() {
	// Error: parameter names must be snake_case, not camelCase
	let _path = path!("/users/{userId}/");
}
