//! URL patterns for the generate app

use reinhardt::ServerRouter;

use super::views;

pub fn url_patterns() -> ServerRouter {
    ServerRouter::new()
        .endpoint(views::health)
        .endpoint(views::generate)
}
