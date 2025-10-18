// Test: URL pattern with typed integer parameter

use reinhardt_macros::path;

fn main() {
    let pattern = path!("polls/{<int:question_id>}/");
    assert_eq!(pattern, "polls/{<int:question_id>}/");
}
