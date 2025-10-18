// Test: URL pattern with unmatched closing brace should fail

use reinhardt_macros::path;

fn main() {
    let pattern = path!("polls/id}/");
}
