//! Component with mixed default children and named slots
// reinhardt-fmt: ignore-all

use reinhardt_pages::page;
use reinhardt_pages::component::Page;

struct LayoutBuilder;
impl LayoutBuilder {
	fn children(self, _children: impl Into<Page>) -> Self { self }
	fn sidebar(self, _children: impl Into<Page>) -> Self { self }
	fn build(self) -> Page { Page::Empty }
}
fn Layout(_args: i32) -> LayoutBuilder { LayoutBuilder }

fn main() {
	let _layout = page!(|| {
		Layout(args: 1) {
			div { "Default child" }
			$sidebar {
				div { "Sidebar content" }
			}
		}
	});
}
