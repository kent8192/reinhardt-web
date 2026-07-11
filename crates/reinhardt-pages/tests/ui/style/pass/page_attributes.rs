use reinhardt_pages::{CssColor, CssLength, page, style_def};

#[style_def]
static STYLES: PageStyles = style! {
	vars {
		accent: Color = red;
		padding: Length = 1rem;
	}
	.card { color: vars.accent; padding: vars.padding; }
	.featured { font-weight: bold; }
};

fn main() {
	let accent = CssColor::parse("blue").unwrap();
	let view = page!({
		div {
			class: STYLES.card() + STYLES.featured() + "legacy-card",
			style: STYLES.vars().accent(accent).padding(CssLength::px(8.0)),
			"Styled"
		}
	});
	let html = view.render_to_string();
	assert!(html.starts_with("<div class=\"card--rs-"));
}
