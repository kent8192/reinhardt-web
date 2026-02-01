//! Admin macro with multiple ordering specifications

use reinhardt_macros::admin;

struct Article;

#[admin(model,
	for = Article,
	name = "Article",
	ordering = [(category, asc), (created_at, desc)]
)]
pub struct ArticleAdmin;

fn main() {
	// Compile test only - verify multiple ordering specs work
}
