use reinhardt_pages::page;
use reinhardt_pages::reactive::Signal;

fn main() {
	let outer = Signal::new(0_i32);
	let _ = page!(|| {
		div { {
			outer.get()
		} }
	});
	let _ = outer;
}
