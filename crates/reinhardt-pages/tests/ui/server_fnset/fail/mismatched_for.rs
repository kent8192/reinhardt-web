include!("../pass/model_crud_types.inc");

struct DeclaredActions;
struct OtherActions;
#[server_fnset(name = "article", actions = DeclaredActions)]
fn article_fns() -> ModelServerFnSet<ArticleResource> { ModelServerFnSet::new() }
#[server_fnset(for = article_fns)]
impl OtherActions {}
fn main() {}
