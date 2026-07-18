include!("../pass/model_crud_types.inc");

struct Actions;
#[server_fnset(name = "article", actions = Actions)]
fn article_fns() -> ModelServerFnSet<ArticleResource> { ModelServerFnSet::new() }
#[server_fnset(for = article_fns)]
impl Actions {}
#[server_fnset(for = article_fns)]
impl Actions {}
fn main() {}
