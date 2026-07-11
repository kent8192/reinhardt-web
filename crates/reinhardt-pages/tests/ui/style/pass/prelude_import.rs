use reinhardt_pages::prelude::*;

#[style_def]
static STYLES: PreludeStyles = style! { .card { color: red; } };

fn main() {
	let _ = STYLES.card();
}
