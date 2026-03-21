//! URL configuration for Rough2Spec example

use reinhardt::UnifiedRouter;
use reinhardt::routes;

use super::views;

#[routes]
pub fn routes() -> UnifiedRouter {
    UnifiedRouter::new()
        .endpoint(views::root)
        .mount("/api/", crate::apps::generate::urls::url_patterns())
}
