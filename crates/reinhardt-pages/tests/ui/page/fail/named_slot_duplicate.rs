//! Duplicate named slot — should fail (E1)
// reinhardt-fmt: ignore-all

use reinhardt_pages::page;

fn main() {
    let _table = page!(|| {
        Table(args: 1) {
            $header { div { "A" } }
            $header { div { "B" } }
        }
    });
}
