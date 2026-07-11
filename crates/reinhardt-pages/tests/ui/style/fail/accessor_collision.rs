use reinhardt_pages::{style_def};

#[style_def]
static STYLES: CardStyles = style! { .foo-bar { color: red; } .foo_bar { color: blue; } };
fn main() {}
