include!("../pass/model_crud_types.inc");

struct Actions;
#[server_fnset(name = "article", actions = Actions)]
fn article_fns() -> ModelServerFnSet<ArticleResource> { ModelServerFnSet::new() }

#[server_fnset(for = article_fns)]
impl Actions {
	#[action(detail = true, transactional = true)]
	async fn publish(
		lookup: i64,
		#[inject] connection: reinhardt_db::orm::DatabaseConnection,
	) -> Result<ArticleDto, ServerFnSetError> {
		let _ = connection;
		Ok(ArticleDto { id: lookup, title: String::new() })
	}
}

fn main() {}
