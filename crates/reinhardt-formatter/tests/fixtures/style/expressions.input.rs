fn styles() {
	let _ = style! {
		vars {
			gutter: Length = 1rem;
			accent: Color = red;
		}
		.card {
			width: 100% - vars.gutter * 2;
			color: vars.accent.mix(white, 15%);
			transform: (translate_x(1rem), rotate(-6deg), scale(1.05));
			filter: unchecked_fn!(var(--accent));
		}
	};
}
