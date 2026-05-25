use reinhardt_pages::page;
use reinhardt_pages::reactive::Signal;

fn main() {
	let _ = page!(|count: Signal<i32>| {
		div {
			watch {
				if count.get()> 0 {
					p {
						"x"
					}
				}
			}
		}
	});
}
