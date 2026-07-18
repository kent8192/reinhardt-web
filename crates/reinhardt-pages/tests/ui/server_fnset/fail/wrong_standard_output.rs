include!("../pass/model_crud_types.inc");

struct Actions;
#[server_fnset(name = "article", actions = Actions)]
fn article_fns() -> ModelServerFnSet<ArticleResource> { ModelServerFnSet::new() }
#[server_fnset(for = article_fns)]
impl Actions {
	async fn update(lookup: i64, input: UpdateArticle, #[inject] context: DetailActionContext<ArticleResource>) -> Result<String, ServerFnSetError> {
		Ok(format!("{lookup}:{}:{}", input.title, context.object().title))
	}
}
fn main() {}
