use reinhardt_pages::{page, reactive::Signal};

fn main() {
	let wrong = Signal::new(false);
	let _ = page!({
		input {
			a11y: off,
			bind: wrong,
		}
	});
}
