// Test: Simple URL pattern without parameters

use reinhardt_macros::path;

fn main() {
    let pattern = path!("polls/");
    assert_eq!(pattern, "polls/");
}
