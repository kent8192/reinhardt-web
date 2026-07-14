include!("model_crud_types.inc");

#[derive(Clone)]
pub struct AuditDependency;

#[async_trait::async_trait]
impl reinhardt_di::Injectable for AuditDependency {
	async fn inject(
		_context: &reinhardt_di::InjectionContext,
	) -> reinhardt_di::DiResult<Self> {
		Ok(Self)
	}
}

struct ArticleActions;

#[server_fnset(name = "article-api", actions = ArticleActions)]
pub fn article_fns() -> ModelServerFnSet<ArticleResource> { ModelServerFnSet::new() }

#[server_fnset(for = article_fns)]
impl ArticleActions {
	pub async fn update(
		lookup: i64,
		input: UpdateArticle,
		authorization: reinhardt_di::params::Header<Option<String>>,
		#[inject] audit: AuditDependency,
		#[inject] mut context: DetailActionContext<ArticleResource>,
	) -> Result<ArticleDto, ServerFnSetError> {
		let _ = (authorization, audit);
		context.object_mut().title = input.title;
		Ok(ArticleDto { id: lookup, title: context.object().title.clone() })
	}

	#[action(detail = true, transactional = true)]
	pub async fn publish(
		lookup: i64,
		input: PublishArticle,
		#[inject] context: DetailActionContext<ArticleResource>,
	) -> Result<ArticleDto, ServerFnSetError> {
		Ok(ArticleDto { id: lookup, title: format!("{}:{}", context.object().title, input.label) })
	}
}

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
async fn assert_all_client_signatures() {
	let _ = article_fns::list(ListQuery).await;
	let _ = article_fns::retrieve(1).await;
	let _ = article_fns::create(CreateArticle { title: String::new() }).await;
	let _ = article_fns::update(1, UpdateArticle { title: String::new() }).await;
	let _ = article_fns::partial_update(1, PatchArticle { title: None }).await;
	let _ = article_fns::destroy(1).await;
	let _ = article_fns::publish(1, PublishArticle { label: String::new() }).await;
}

fn main() {
	let metadata = article_fns().metadata();
	let actual: Vec<_> = metadata
		.actions
		.iter()
		.map(|action| (action.path, action.name, action.detail, action.transactional))
		.collect();
	assert_eq!(metadata.name, "article-api");
	assert_eq!(
		actual,
		vec![
			("/api/server_fn/article-api/list", "article-api-list", false, false),
			("/api/server_fn/article-api/retrieve", "article-api-retrieve", true, false),
			("/api/server_fn/article-api/create", "article-api-create", false, true),
			("/api/server_fn/article-api/update", "article-api-update", true, true),
			("/api/server_fn/article-api/partial-update", "article-api-partial-update", true, true),
			("/api/server_fn/article-api/destroy", "article-api-destroy", true, true),
			("/api/server_fn/article-api/publish", "article-api-publish", true, true),
		],
	);
	let _ = (
		article_fns::list,
		article_fns::retrieve,
		article_fns::create,
		article_fns::update,
		article_fns::partial_update,
		article_fns::destroy,
		article_fns::publish,
	);
}
