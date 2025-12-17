//! UI components module

pub mod common;
pub mod dashboard;
pub mod detail_view;
pub mod filters;
pub mod form;
pub mod list_view;
pub mod pagination;

pub use dashboard::render as render_dashboard;
pub use detail_view::render as render_detail_view;
pub use form::render as render_form;
pub use list_view::render as render_list_view;
