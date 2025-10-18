// Test: URL pattern with empty parameter should fail

use reinhardt_macros::path;

fn main() {
    let pattern = path!("polls/{}/");
}
