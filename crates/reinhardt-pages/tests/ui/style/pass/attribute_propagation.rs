use reinhardt_pages::{style_def};

#[doc = "Generated style API"]
#[allow(non_upper_case_globals)]
#[style_def]
static styles: DocumentedStyles = style! {
	.card { color: red; }
};

fn main() {
	let _ = styles.card();
}
