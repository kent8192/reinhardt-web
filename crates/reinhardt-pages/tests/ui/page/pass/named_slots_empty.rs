//! Component with an empty named slot
// reinhardt-fmt: ignore-all

use reinhardt_pages::page;
use reinhardt_pages::component::Page;

struct TableBuilder;
impl TableBuilder {
	fn header(self, _children: impl Into<Page>) -> Self { self }
	fn build(self) -> Page { Page::Empty }
}
fn Table(_args: i32) -> TableBuilder { TableBuilder }

fn main() {
	let _table: Page = page!(|| {
		Table(args: 1) {
			$header {}
		}
	});
}
