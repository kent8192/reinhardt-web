use reinhardt_pages::{style_def};

#[style_def]
static STYLES: CardStyles = style! {
	.card {
		color: 1px.mix(red, 10%);
	}
};
fn main() {}
