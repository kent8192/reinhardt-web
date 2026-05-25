//! Named slot outside component body — should fail (E3)
// reinhardt-fmt: ignore-all

use reinhardt_pages::page;

fn main() {
    let _page = page!(|| {
        div {
            $header { "This is invalid" }
        }
    });
}
