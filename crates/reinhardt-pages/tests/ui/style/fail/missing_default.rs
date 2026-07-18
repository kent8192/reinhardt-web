use reinhardt_pages::{style_def};

#[style_def]
static STYLES: CardStyles = style! {
	vars {
		accent: Color;
	}
	.card {
		color: red;
	}
};
fn main() {}
