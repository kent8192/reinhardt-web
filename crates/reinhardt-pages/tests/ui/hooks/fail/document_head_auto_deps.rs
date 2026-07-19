//! Compile-fail: retained document-head hooks require explicit dependencies.

use reinhardt_pages::deps_auto;
use reinhardt_pages::reactive::hooks::use_page_title;

fn main() {
	use_page_title(|| "Document title", deps_auto!());
}
