use reinhardt_routers_macros::path;

fn main() {
	let _ = path!("/users/{id}/");
	let _ = path!("/users/{user_id}/");
	let _ = path!("/users/{user_id}/posts/{post_id}/");
	let _ = path!("/items/{item_id}/details/");
}
