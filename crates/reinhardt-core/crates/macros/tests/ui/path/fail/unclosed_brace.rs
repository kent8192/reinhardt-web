// Test: URL pattern with unclosed brace should fail

use reinhardt_macros::path;

fn main() {
    let pattern = path!("polls/{id");
}
