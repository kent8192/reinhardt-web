include!("model_crud_types.inc");

pub(crate) mod actions {
	pub(crate) struct ArticleActions;
}

pub(crate) mod api {
	use super::*;

	#[server_fnset(name = "qualified-article-api", actions = super::actions::ArticleActions)]
	pub(crate) fn article_fns() -> ModelServerFnSet<ArticleResource> {
		ModelServerFnSet::new()
	}
}

#[server_fnset(for = api::article_fns)]
impl actions::ArticleActions {
	#[action(detail = false, transactional = false)]
	pub(crate) async fn collision(
		connection: String,
		principal: String,
		#[inject] _context: reinhardt_pages::server_fn::CollectionReadActionContext<ArticleResource>,
	) -> Result<ArticleDto, ServerFnSetError> {
		Ok(ArticleDto {
			id: 0,
			title: format!("{connection}:{principal}"),
		})
	}
}

fn main() {
	let metadata = api::article_fns().metadata();
	assert_eq!(metadata.name, "qualified-article-api");
	let _ = article_fns::collision;
}
