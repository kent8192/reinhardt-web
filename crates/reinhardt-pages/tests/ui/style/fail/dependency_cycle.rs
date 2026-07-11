use reinhardt_pages::{style_def};

#[style_def]
static STYLES: CardStyles = style! { vars { first: Color = vars.second; second: Color = vars.first; } .card { color: vars.first; } };
fn main() {}
