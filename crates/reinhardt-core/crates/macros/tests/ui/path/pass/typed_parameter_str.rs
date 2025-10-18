// Test: URL pattern with typed string parameter

use reinhardt_macros::path;

fn main() {
    let pattern = path!("articles/{<str:slug>}/");
    assert_eq!(pattern, "articles/{<str:slug>}/");
}
