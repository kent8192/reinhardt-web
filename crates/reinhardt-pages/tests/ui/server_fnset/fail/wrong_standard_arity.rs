include!("../pass/model_crud_types.inc");

struct Actions;
#[server_fnset(name = "article", actions = Actions)]
fn article_fns() -> ModelServerFnSet<ArticleResource> { ModelServerFnSet::new() }

#[server_fnset(for = article_fns)]
impl Actions {
	async fn update(
		lookup: i64,
		#[inject] context: DetailActionContext<ArticleResource>,
	) -> Result<ArticleDto, ServerFnSetError> {
		Ok(ArticleDto { id: lookup, title: context.object().title.clone() })
	}
}

fn main() {}
