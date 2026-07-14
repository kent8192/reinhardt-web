#![cfg(all(target_family = "wasm", target_os = "unknown"))]
// This compile-only target validates generated signatures without executing them.
#![allow(dead_code)]

use std::marker::PhantomData;

use reinhardt_pages::server_fn::{
	PageRequest, ServerFnListQuery, ServerFnResource, ServerFnSetError, server_fnset,
};
use serde::{Deserialize, Serialize};

pub struct ModelServerFnSet<R>(PhantomData<R>);

impl<R> ModelServerFnSet<R> {
	fn new() -> Self {
		Self(PhantomData)
	}
}

pub struct DetailActionContext<'a, R>(PhantomData<&'a R>);

#[derive(Clone, Deserialize, Serialize)]
pub struct ListQuery;

impl ServerFnListQuery for ListQuery {
	fn page_request(&self) -> PageRequest {
		PageRequest::default()
	}
}

#[derive(Clone, Deserialize, Serialize)]
pub struct ArticleDto;

#[derive(Clone, Deserialize, Serialize)]
pub struct CreateArticle;

#[derive(Clone, Deserialize, Serialize)]
pub struct UpdateArticle;

#[derive(Clone, Deserialize, Serialize)]
pub struct PatchArticle;

#[derive(Clone, Deserialize, Serialize)]
pub struct PublishArticle;

pub struct ArticleResource;

impl ServerFnResource for ArticleResource {
	type Lookup = i64;
	type Read = ArticleDto;
	type Create = CreateArticle;
	type Update = UpdateArticle;
	type Patch = PatchArticle;
	type ListQuery = ListQuery;
}

pub struct AuditDependency;
pub struct Header<T>(PhantomData<T>);
pub struct ArticleActions;

#[server_fnset(name = "article-api", actions = ArticleActions)]
pub fn article_fns() -> ModelServerFnSet<ArticleResource> {
	ModelServerFnSet::new()
}

#[server_fnset(for = article_fns)]
impl ArticleActions {
	async fn update(
		lookup: i64,
		input: UpdateArticle,
		authorization: Header<Option<String>>,
		#[inject] audit: AuditDependency,
		#[inject] context: DetailActionContext<ArticleResource>,
	) -> Result<ArticleDto, ServerFnSetError> {
		let _ = (lookup, input, authorization, audit, context);
		Ok(ArticleDto)
	}

	#[action(detail = true, transactional = true)]
	async fn publish(
		lookup: i64,
		input: PublishArticle,
		#[inject] context: DetailActionContext<ArticleResource>,
	) -> Result<ArticleDto, ServerFnSetError> {
		let _ = (lookup, input, context);
		Ok(ArticleDto)
	}
}

async fn assert_all_client_signatures() {
	let _ = article_fns::list(ListQuery).await;
	let _ = article_fns::retrieve(1).await;
	let _ = article_fns::create(CreateArticle).await;
	let _ = article_fns::update(1, UpdateArticle).await;
	let _ = article_fns::partial_update(1, PatchArticle).await;
	let _ = article_fns::destroy(1).await;
	let _ = article_fns::publish(1, PublishArticle).await;
}
