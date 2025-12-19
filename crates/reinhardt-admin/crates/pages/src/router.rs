//! Client-side router for Reinhardt Admin Panel
//!
//! Handles routing between different admin pages:
//! - `/admin/` - Dashboard
//! - `/admin/:model/` - List view
//! - `/admin/:model/:id/` - Detail view
//! - `/admin/:model/add/` - Create form
//! - `/admin/:model/:id/change/` - Edit form

use crate::components::features::{
	Column, DashboardModel, FormField, ListViewData, dashboard, detail_view, list_view, model_form,
};
use reinhardt_pages::Signal;
use reinhardt_pages::component::{Component, View};
use reinhardt_pages::router::{Link, Router};
use std::collections::HashMap;

/// Admin route enum
#[derive(Debug, Clone, PartialEq)]
pub enum AdminRoute {
	/// Dashboard route
	Dashboard,
	/// List view route
	List { model_name: String },
	/// Detail view route
	Detail { model_name: String, id: String },
	/// Create form route
	Create { model_name: String },
	/// Edit form route
	Edit { model_name: String, id: String },
	/// Not found route
	NotFound,
}

// Global route state signals
// These are populated by the router when navigation occurs
thread_local! {
	static CURRENT_MODEL: Signal<Option<String>> = Signal::new(None);
	static CURRENT_ID: Signal<Option<String>> = Signal::new(None);
}

/// Dashboard view component for router
fn dashboard_view() -> View {
	// TODO: Fetch actual models from AdminSite via Server Function
	let models = vec![
		DashboardModel {
			name: "Users".to_string(),
			url: "/admin/users/".to_string(),
		},
		DashboardModel {
			name: "Posts".to_string(),
			url: "/admin/posts/".to_string(),
		},
	];

	dashboard("Admin Panel", &models)
}

/// List view component for router
fn list_view_component() -> View {
	let model_name = CURRENT_MODEL.with(|s| s.get().unwrap_or_else(|| "unknown".to_string()));

	// TODO: Fetch actual data via Server Function
	let data = ListViewData {
		model_name: model_name.clone(),
		columns: vec![
			Column {
				field: "id".to_string(),
				label: "ID".to_string(),
				sortable: true,
			},
			Column {
				field: "name".to_string(),
				label: "Name".to_string(),
				sortable: true,
			},
		],
		records: vec![],
		current_page: 1,
		total_pages: 1,
		total_count: 0,
	};

	let page_signal = Signal::new(1u64);
	list_view(&data, page_signal)
}

/// Detail view component for router
fn detail_view_component() -> View {
	let model_name = CURRENT_MODEL.with(|s| s.get().unwrap_or_else(|| "unknown".to_string()));
	let record_id = CURRENT_ID.with(|s| s.get().unwrap_or_else(|| "0".to_string()));

	// TODO: Fetch actual record via Server Function
	let mut record = HashMap::new();
	record.insert("id".to_string(), record_id.clone());
	record.insert("name".to_string(), "Sample Record".to_string());

	detail_view(&model_name, &record_id, &record)
}

/// Create form view component for router
fn create_view_component() -> View {
	let model_name = CURRENT_MODEL.with(|s| s.get().unwrap_or_else(|| "unknown".to_string()));

	// TODO: Fetch field definitions from ModelAdmin via Server Function
	let fields = vec![
		FormField {
			name: "name".to_string(),
			label: "Name".to_string(),
			field_type: "text".to_string(),
			required: true,
			value: String::new(),
		},
		FormField {
			name: "email".to_string(),
			label: "Email".to_string(),
			field_type: "email".to_string(),
			required: true,
			value: String::new(),
		},
	];

	model_form(&model_name, &fields, None)
}

/// Edit form view component for router
fn edit_view_component() -> View {
	let model_name = CURRENT_MODEL.with(|s| s.get().unwrap_or_else(|| "unknown".to_string()));
	let record_id = CURRENT_ID.with(|s| s.get().unwrap_or_else(|| "0".to_string()));

	// TODO: Fetch field definitions and record data via Server Function
	let fields = vec![
		FormField {
			name: "name".to_string(),
			label: "Name".to_string(),
			field_type: "text".to_string(),
			required: true,
			value: "Existing Value".to_string(),
		},
		FormField {
			name: "email".to_string(),
			label: "Email".to_string(),
			field_type: "email".to_string(),
			required: true,
			value: "user@example.com".to_string(),
		},
	];

	model_form(&model_name, &fields, Some(&record_id))
}

/// Not found view component for router
fn not_found_view() -> View {
	use reinhardt_pages::component::{ElementView, IntoView};

	ElementView::new("div")
		.attr("class", "not-found")
		.child(
			ElementView::new("h1")
				.attr("class", "text-center mt-5")
				.child("404 - Page Not Found"),
		)
		.child(
			ElementView::new("p")
				.attr("class", "text-center")
				.child("The requested page could not be found."),
		)
		.child(
			ElementView::new("div")
				.attr("class", "text-center mt-3")
				.child(Link::new("/admin/", "Go to Dashboard").render()),
		)
		.into_view()
}

/// Initialize the admin router
///
/// # Example
///
/// ```ignore
/// use reinhardt_admin_pages::router::init_router;
///
/// let router = init_router();
/// ```
pub fn init_router() -> Router {
	Router::new()
		.named_route("dashboard", "/admin/", dashboard_view)
		.named_route("list", "/admin/{model}/", || {
			// Extract model parameter from URL
			// TODO: Get actual params from Router context
			list_view_component()
		})
		.named_route("detail", "/admin/{model}/{id}/", || {
			// Extract model and id parameters from URL
			// TODO: Get actual params from Router context
			detail_view_component()
		})
		.named_route("create", "/admin/{model}/add/", || {
			// Extract model parameter from URL
			// TODO: Get actual params from Router context
			create_view_component()
		})
		.named_route("edit", "/admin/{model}/{id}/change/", || {
			// Extract model and id parameters from URL
			// TODO: Get actual params from Router context
			edit_view_component()
		})
		.not_found(not_found_view)
}

/// Updates route state signals when navigation occurs
///
/// This should be called by the router navigation handler
pub fn update_route_state(model: Option<String>, id: Option<String>) {
	CURRENT_MODEL.with(|s| s.set(model));
	CURRENT_ID.with(|s| s.set(id));
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_admin_route_enum() {
		let route = AdminRoute::Dashboard;
		assert_eq!(route, AdminRoute::Dashboard);

		let route = AdminRoute::List {
			model_name: "users".to_string(),
		};
		assert!(matches!(route, AdminRoute::List { .. }));
	}

	#[test]
	fn test_init_router_creates_routes() {
		let router = init_router();
		assert_eq!(router.route_count(), 5); // dashboard + list + detail + create + edit
		assert!(router.has_route("dashboard"));
		assert!(router.has_route("list"));
		assert!(router.has_route("detail"));
		assert!(router.has_route("create"));
		assert!(router.has_route("edit"));
	}

	#[test]
	fn test_dashboard_route_match() {
		let router = init_router();
		let route_match = router.match_path("/admin/");
		assert!(route_match.is_some());

		let route_match = route_match.unwrap();
		assert_eq!(route_match.route.name(), Some("dashboard"));
	}

	#[test]
	fn test_list_route_match() {
		let router = init_router();
		let route_match = router.match_path("/admin/users/");
		assert!(route_match.is_some());

		let route_match = route_match.unwrap();
		assert_eq!(route_match.route.name(), Some("list"));
		assert_eq!(route_match.params.get("model"), Some(&"users".to_string()));
	}

	#[test]
	fn test_detail_route_match() {
		let router = init_router();
		let route_match = router.match_path("/admin/users/42/");
		assert!(route_match.is_some());

		let route_match = route_match.unwrap();
		assert_eq!(route_match.route.name(), Some("detail"));
		assert_eq!(route_match.params.get("model"), Some(&"users".to_string()));
		assert_eq!(route_match.params.get("id"), Some(&"42".to_string()));
	}

	#[test]
	fn test_create_route_match() {
		let router = init_router();
		let route_match = router.match_path("/admin/users/add/");
		assert!(route_match.is_some());

		let route_match = route_match.unwrap();
		assert_eq!(route_match.route.name(), Some("create"));
		assert_eq!(route_match.params.get("model"), Some(&"users".to_string()));
	}

	#[test]
	fn test_edit_route_match() {
		let router = init_router();
		let route_match = router.match_path("/admin/users/42/change/");
		assert!(route_match.is_some());

		let route_match = route_match.unwrap();
		assert_eq!(route_match.route.name(), Some("edit"));
		assert_eq!(route_match.params.get("model"), Some(&"users".to_string()));
		assert_eq!(route_match.params.get("id"), Some(&"42".to_string()));
	}

	#[test]
	fn test_reverse_url_dashboard() {
		let router = init_router();
		let url = router.reverse("dashboard", &[]).unwrap();
		assert_eq!(url, "/admin/");
	}

	#[test]
	fn test_reverse_url_list() {
		let router = init_router();
		let url = router.reverse("list", &[("model", "users")]).unwrap();
		assert_eq!(url, "/admin/users/");
	}

	#[test]
	fn test_reverse_url_detail() {
		let router = init_router();
		let url = router
			.reverse("detail", &[("model", "users"), ("id", "42")])
			.unwrap();
		assert_eq!(url, "/admin/users/42/");
	}

	#[test]
	fn test_update_route_state() {
		update_route_state(Some("users".to_string()), Some("42".to_string()));

		let model = CURRENT_MODEL.with(|s| s.get());
		let id = CURRENT_ID.with(|s| s.get());

		assert_eq!(model, Some("users".to_string()));
		assert_eq!(id, Some("42".to_string()));
	}

	#[test]
	fn test_update_route_state_none() {
		update_route_state(None, None);

		let model = CURRENT_MODEL.with(|s| s.get());
		let id = CURRENT_ID.with(|s| s.get());

		assert_eq!(model, None);
		assert_eq!(id, None);
	}
}
