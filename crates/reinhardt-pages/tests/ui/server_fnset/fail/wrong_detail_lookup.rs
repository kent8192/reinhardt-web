include!("../pass/model_crud_types.inc");

struct Actions;
#[server_fnset(name = "article", actions = Actions)]
fn article_fns() -> ModelServerFnSet<ArticleResource> { ModelServerFnSet::new() }
#[server_fnset(for = article_fns)]
impl Actions {
	#[action(detail = true)]
	async fn preview(lookup: String, #[inject] context: reinhardt_pages::server_fn::DetailReadActionContext<ArticleResource>) -> Result<ArticleDto, ServerFnSetError> {
		Ok(ArticleDto { id: lookup.len() as i64, title: context.object().title.clone() })
	}
}
fn main() {}
