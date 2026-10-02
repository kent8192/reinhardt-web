include!("model_crud_types.inc");

use reinhardt_pages::server_fn::CollectionReadActionContext;

pub mod admin {
	use super::*;

	pub struct ArticleActions;

	#[server_fnset(name = "admin-article-api", actions = ArticleActions)]
	pub fn article_fns() -> ModelServerFnSet<ArticleResource> {
		ModelServerFnSet::new()
	}
}

pub mod public {
	use super::*;

	pub struct ArticleActions;

	#[server_fnset(name = "public-article-api", actions = ArticleActions)]
	pub(super) fn article_fns() -> ModelServerFnSet<ArticleResource> {
		ModelServerFnSet::new()
	}
}

pub(crate) mod scoped {
	use super::*;

	pub struct ScopedActions;

	pub(crate) mod api {
		use super::*;

		#[server_fnset(name = "scoped-article-api", actions = super::ScopedActions)]
		pub(super) fn article_fns() -> ModelServerFnSet<ArticleResource> {
			ModelServerFnSet::new()
		}

		pub(crate) fn metadata_name() -> &'static str {
			article_fns().metadata().name
		}
	}
}

#[server_fnset(for = scoped::api::article_fns)]
impl scoped::ScopedActions {}

#[server_fnset(for = admin::article_fns)]
impl admin::ArticleActions {
	#[action(detail = false, transactional = false)]
	pub(crate) async fn echo(
		context: String,
		#[inject] _action_context: CollectionReadActionContext<ArticleResource>,
	) -> Result<ArticleDto, ServerFnSetError> {
		Ok(ArticleDto {
			id: 0,
			title: context,
		})
	}
}

#[server_fnset(for = public::article_fns)]
impl public::ArticleActions {}

fn main() {
	let admin_metadata = admin::article_fns().metadata();
	let public_metadata = public::article_fns().metadata();
	assert_eq!(admin_metadata.name, "admin-article-api");
	assert_eq!(public_metadata.name, "public-article-api");
	assert_eq!(scoped::api::metadata_name(), "scoped-article-api");
}
