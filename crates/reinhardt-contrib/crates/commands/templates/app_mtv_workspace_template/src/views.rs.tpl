//! Views for {{ app_name }}

// Re-export views from views module
pub use self::views::*;

pub mod views {
    use reinhardt_core::{Request, Response};

    // Define your views here
    // Example:
    // pub async fn index(req: Request) -> Response {
    //     Response::render("{{ app_name }}/index.html", context! {})
    // }
}
