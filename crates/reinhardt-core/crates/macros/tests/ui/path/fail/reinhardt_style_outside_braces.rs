// Test: Django-style parameters outside braces should fail

use reinhardt_macros::path;

fn main() {
    let pattern = path!("polls/<int:id>/");
}
