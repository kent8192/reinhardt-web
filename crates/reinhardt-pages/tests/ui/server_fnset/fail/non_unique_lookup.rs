include!("../pass/model_crud_types.inc");

fn require_non_unique_accessor() -> UniqueFieldRef<Article, String> {
	Article::unique_title()
}
fn main() { let _ = require_non_unique_accessor(); }
