// Test: URL pattern with underscored parameter names

use reinhardt_macros::path;

fn main() {
	let pattern = path!("users/{user_id}/posts/{post_id}/comments/{comment_id}/");
	assert_eq!(
		pattern,
		"users/{user_id}/posts/{post_id}/comments/{comment_id}/"
	);
}
