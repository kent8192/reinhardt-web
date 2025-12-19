//! Layout components
//!
//! Provides layout components for the admin panel:
//! - `Header` - Top navigation bar
//! - `Sidebar` - Side navigation menu
//! - `Footer` - Footer component
//! - `MainLayout` - Main layout wrapper

use reinhardt_pages::component::{ElementView, IntoView, View};

/// Model information for navigation
#[derive(Debug, Clone)]
pub struct ModelInfo {
	/// Model name (display name)
	pub name: String,
	/// URL path for the model list view
	pub url: String,
}

/// Header component
///
/// Displays the top navigation bar with site title and user menu.
///
/// # Example
///
/// ```ignore
/// use reinhardt_admin_pages::components::layout::header;
///
/// header("My Admin Panel", Some("john_doe"))
/// ```
pub fn header(site_name: &str, user_name: Option<&str>) -> View {
	let user_display = user_name.unwrap_or("Guest");

	ElementView::new("nav")
		.attr("class", "navbar navbar-dark bg-dark")
		.child(
			ElementView::new("div")
				.attr("class", "container-fluid")
				.child(
					ElementView::new("a")
						.attr("class", "navbar-brand")
						.attr("href", "/admin/")
						.child(site_name),
				)
				.child(
					ElementView::new("span")
						.attr("class", "navbar-text")
						.child(format!("User: {}", user_display)),
				),
		)
		.into_view()
}

/// Sidebar component
///
/// Displays the side navigation menu with model links.
/// Uses Link component for SPA navigation.
///
/// # Example
///
/// ```ignore
/// use reinhardt_admin_pages::components::layout::{sidebar, ModelInfo};
///
/// let models = vec![
///     ModelInfo { name: "Users".to_string(), url: "/admin/users/".to_string() },
///     ModelInfo { name: "Posts".to_string(), url: "/admin/posts/".to_string() },
/// ];
/// sidebar(&models, Some("/admin/users/"))
/// ```
pub fn sidebar(models: &[ModelInfo], current_path: Option<&str>) -> View {
	use reinhardt_pages::component::Component;
	use reinhardt_pages::router::Link;

	let nav_items: Vec<View> = models
		.iter()
		.map(|model| {
			let is_active = current_path.is_some_and(|path| path.starts_with(&model.url));
			let item_class = if is_active {
				"nav-link active"
			} else {
				"nav-link"
			};

			ElementView::new("li")
				.attr("class", "nav-item")
				.child(
					Link::new(model.url.clone(), model.name.clone())
						.class(item_class)
						.render(),
				)
				.into_view()
		})
		.collect();

	ElementView::new("div")
		.attr("class", "sidebar bg-light border-end")
		.attr(
			"style",
			"width: 250px; height: 100vh; position: fixed; top: 56px; left: 0; overflow-y: auto;",
		)
		.child(
			ElementView::new("ul")
				.attr("class", "nav flex-column")
				.children(nav_items),
		)
		.into_view()
}

/// Footer component
///
/// Displays the footer with copyright and version information.
///
/// # Example
///
/// ```ignore
/// use reinhardt_admin_pages::components::layout::footer;
///
/// footer("0.1.0")
/// ```
pub fn footer(version: &str) -> View {
	ElementView::new("footer")
		.attr("class", "footer bg-light text-center py-3 border-top")
		.attr("style", "margin-left: 250px;")
		.child(
			ElementView::new("div")
				.attr("class", "container-fluid")
				.child(format!("Reinhardt Admin Panel v{}", version)),
		)
		.into_view()
}

/// Main layout wrapper
///
/// Wraps the main content area with header, sidebar, and footer.
/// Uses RouterOutlet for dynamic content rendering.
///
/// # Example
///
/// ```ignore
/// use reinhardt_admin_pages::components::layout::{main_layout, ModelInfo};
///
/// let models = vec![
///     ModelInfo { name: "Users".to_string(), url: "/admin/users/".to_string() },
/// ];
/// main_layout("My Admin", &models, None, "0.1.0")
/// ```
pub fn main_layout(
	site_name: &str,
	models: &[ModelInfo],
	user_name: Option<&str>,
	version: &str,
) -> View {
	use reinhardt_pages::component::Component;
	use reinhardt_pages::router::RouterOutlet;

	ElementView::new("div")
		.attr("class", "admin-layout")
		.child(header(site_name, user_name))
		.child(sidebar(models, None))
		.child(
			ElementView::new("main")
				.attr("class", "main-content")
				.attr(
					"style",
					"margin-left: 250px; margin-top: 56px; padding: 20px; min-height: calc(100vh - 120px);",
				)
				.child(
					RouterOutlet::new()
						.id("admin-outlet")
						.class("router-content")
						.render(),
				),
		)
		.child(footer(version))
		.into_view()
}
