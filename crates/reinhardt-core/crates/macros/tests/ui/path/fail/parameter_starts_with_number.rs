// Test: URL pattern with parameter starting with number should fail

use reinhardt_macros::path;

fn main() {
    let pattern = path!("polls/{1id}/");
}
