// Test: URL pattern with multiple parameters

use reinhardt_macros::path;

fn main() {
    let pattern = path!("users/{user_id}/posts/{post_id}/");
    assert_eq!(pattern, "users/{user_id}/posts/{post_id}/");
}
