include!("../pass/model_crud_types.inc");

struct Actions;
#[server_fnset(name = "article", actions = Actions)]
fn article_fns() -> ModelServerFnSet<ArticleResource> { ModelServerFnSet::new() }

#[server_fnset(for = article_fns)]
impl Actions {
	#[action(detail = true, transactional = true)]
	async fn update(lookup: i64, input: UpdateArticle, #[inject] context: DetailActionContext<ArticleResource>) -> Result<ArticleDto, ServerFnSetError> {
		Ok(ArticleDto { id: lookup, title: input.title + &context.object().title })
	}
}

fn main() {}
