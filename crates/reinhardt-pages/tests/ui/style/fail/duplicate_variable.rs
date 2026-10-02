use reinhardt_pages::{style_def};

#[style_def]
static STYLES: CardStyles = style! {
	vars {
		accent: Color = red;
		accent: Color = blue;
	}
	.card {
		color: red;
	}
};
fn main() {}
