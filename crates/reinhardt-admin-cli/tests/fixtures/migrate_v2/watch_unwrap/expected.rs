use reinhardt_pages::page;
use reinhardt_pages::reactive::Signal;
fn r(count: Signal<i32>) {
    let _ = page!(|count: Signal<i32>| {
	div {
		if count.get()> 0 {
			p {
				"positive"
			}
		} else {
			p {
				"non-positive"
			}
		}
	}
})(count);
}
