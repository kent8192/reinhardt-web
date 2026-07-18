include!("model_crud_types.inc");

use reinhardt_pages::server_fn::{
	CollectionReadActionContext, CreateActionContext, DetailReadActionContext, Page,
};

struct ArticleActions;

#[server_fnset(name = "article-overrides", actions = ArticleActions)]
pub fn article_fns() -> ModelServerFnSet<ArticleResource> {
    ModelServerFnSet::new()
}

#[server_fnset(for = article_fns)]
impl ArticleActions {
    pub async fn list(
        _query: ListQuery,
        #[inject] context: CollectionReadActionContext<ArticleResource>,
    ) -> Result<Page<ArticleDto>, ServerFnSetError> {
        let _ = context.queryset();
        Ok(Page {
            items: Vec::new(),
            total: 0,
            limit: 25,
            offset: 0,
        })
    }

    pub async fn retrieve(
        lookup: i64,
        #[inject] context: DetailReadActionContext<ArticleResource>,
    ) -> Result<ArticleDto, ServerFnSetError> {
        Ok(ArticleDto {
            id: lookup,
            title: context.object().title.clone(),
        })
    }

    pub async fn create(
        input: CreateArticle,
        #[inject] mut context: CreateActionContext<ArticleResource>,
    ) -> Result<ArticleDto, ServerFnSetError> {
        let _ = context.executor_mut();
        Ok(ArticleDto {
            id: 0,
            title: input.title,
        })
    }

    pub async fn update(
        lookup: i64,
        input: UpdateArticle,
        #[inject] mut context: DetailActionContext<ArticleResource>,
    ) -> Result<ArticleDto, ServerFnSetError> {
        context.object_mut().title = input.title;
        Ok(ArticleDto {
            id: lookup,
            title: context.object().title.clone(),
        })
    }

    pub async fn partial_update(
        lookup: i64,
        input: PatchArticle,
        #[inject] mut context: DetailActionContext<ArticleResource>,
    ) -> Result<ArticleDto, ServerFnSetError> {
        if let Some(title) = input.title {
            context.object_mut().title = title;
        }
        Ok(ArticleDto {
            id: lookup,
            title: context.object().title.clone(),
        })
    }

    pub async fn destroy(
        _lookup: i64,
        #[inject] _context: DetailActionContext<ArticleResource>,
    ) -> Result<(), ServerFnSetError> {
        Ok(())
    }
}

fn main() {
    let metadata = article_fns().metadata();
    assert_eq!(metadata.actions.len(), 6);
    let _ = (
        article_fns::list,
        article_fns::retrieve,
        article_fns::create,
        article_fns::update,
        article_fns::partial_update,
        article_fns::destroy,
    );
}
