//! Dashboard widget system for admin panel
//!
//! This module provides a flexible widget system for displaying statistics,
//! recent activities, charts, and custom information on the admin dashboard.

use crate::{AdminError, AdminResult};
use async_trait::async_trait;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Position of a widget on the dashboard
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WidgetPosition {
	/// Top left position
	TopLeft,
	/// Top right position
	TopRight,
	/// Center position
	Center,
	/// Bottom left position
	BottomLeft,
	/// Bottom right position
	BottomRight,
}

/// Widget trait for dashboard components
#[async_trait]
pub trait DashboardWidget: Send + Sync {
	/// Get the widget title
	fn title(&self) -> &str;

	/// Get the widget icon (optional)
	fn icon(&self) -> Option<&str> {
		None
	}

	/// Get the widget position
	fn position(&self) -> WidgetPosition;

	/// Get the widget size (width, height) in grid units
	fn size(&self) -> (u32, u32) {
		(1, 1)
	}

	/// Get the widget's refresh interval in seconds (None = no auto-refresh)
	fn refresh_interval(&self) -> Option<u32> {
		None
	}

	/// Check if the widget is visible to the current user
	async fn is_visible(&self, _user_permissions: &[String]) -> bool {
		true
	}

	/// Render the widget to HTML
	async fn render(&self, context: &WidgetContext) -> AdminResult<String>;

	/// Load widget data asynchronously
	async fn load_data(&self) -> AdminResult<serde_json::Value> {
		Ok(serde_json::json!({}))
	}
}

/// Context for rendering widgets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetContext {
	/// User information
	pub user: Option<UserInfo>,
	/// Additional context data
	pub extra: HashMap<String, serde_json::Value>,
}

impl WidgetContext {
	/// Create a new widget context
	pub fn new() -> Self {
		Self {
			user: None,
			extra: HashMap::new(),
		}
	}

	/// Set user information
	pub fn with_user(mut self, user: UserInfo) -> Self {
		self.user = Some(user);
		self
	}

	/// Add extra context data
	pub fn with_extra(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
		self.extra.insert(key.into(), value);
		self
	}
}

impl Default for WidgetContext {
	fn default() -> Self {
		Self::new()
	}
}

/// User information for widget context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
	pub username: String,
	pub permissions: Vec<String>,
}

/// Configuration for a widget
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetConfig {
	/// Widget ID
	pub id: String,
	/// Widget position
	pub position: WidgetPosition,
	/// Widget size (width, height) in grid units
	pub size: (u32, u32),
	/// Widget order (for sorting within same position)
	pub order: i32,
	/// Custom CSS classes
	pub css_classes: Vec<String>,
	/// Custom styling
	pub style: HashMap<String, String>,
	/// Widget-specific options
	pub options: HashMap<String, serde_json::Value>,
}

impl WidgetConfig {
	/// Create a new widget configuration
	pub fn new(id: impl Into<String>, position: WidgetPosition) -> Self {
		Self {
			id: id.into(),
			position,
			size: (1, 1),
			order: 0,
			css_classes: Vec::new(),
			style: HashMap::new(),
			options: HashMap::new(),
		}
	}

	/// Set widget size
	pub fn with_size(mut self, width: u32, height: u32) -> Self {
		self.size = (width, height);
		self
	}

	/// Set widget order
	pub fn with_order(mut self, order: i32) -> Self {
		self.order = order;
		self
	}

	/// Add a CSS class
	pub fn add_class(mut self, class: impl Into<String>) -> Self {
		self.css_classes.push(class.into());
		self
	}

	/// Add a style property
	pub fn add_style(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
		self.style.insert(key.into(), value.into());
		self
	}

	/// Add an option
	pub fn add_option(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
		self.options.insert(key.into(), value);
		self
	}
}

/// Registry for dashboard widgets
pub struct WidgetRegistry {
	widgets: DashMap<String, (Arc<dyn DashboardWidget>, WidgetConfig)>,
}

impl WidgetRegistry {
	/// Create a new widget registry
	pub fn new() -> Self {
		Self {
			widgets: DashMap::new(),
		}
	}

	/// Register a widget
	pub fn register(
		&self,
		widget: Arc<dyn DashboardWidget>,
		config: WidgetConfig,
	) -> AdminResult<()> {
		let id = config.id.clone();
		if self.widgets.contains_key(&id) {
			return Err(AdminError::ValidationError(format!(
				"Widget '{}' is already registered",
				id
			)));
		}
		self.widgets.insert(id, (widget, config));
		Ok(())
	}

	/// Unregister a widget
	pub fn unregister(&self, id: &str) -> AdminResult<()> {
		if self.widgets.remove(id).is_none() {
			return Err(AdminError::ValidationError(format!(
				"Widget '{}' is not registered",
				id
			)));
		}
		Ok(())
	}

	/// Get a widget by ID
	pub fn get(&self, id: &str) -> Option<(Arc<dyn DashboardWidget>, WidgetConfig)> {
		self.widgets.get(id).map(|entry| entry.value().clone())
	}

	/// Get all widgets for a specific position
	pub fn get_by_position(
		&self,
		position: WidgetPosition,
	) -> Vec<(Arc<dyn DashboardWidget>, WidgetConfig)> {
		let mut widgets: Vec<_> = self
			.widgets
			.iter()
			.filter(|entry| entry.value().1.position == position)
			.map(|entry| entry.value().clone())
			.collect();

		// Sort by order
		widgets.sort_by_key(|(_, config)| config.order);
		widgets
	}

	/// Get all widgets
	pub fn all(&self) -> Vec<(Arc<dyn DashboardWidget>, WidgetConfig)> {
		self.widgets
			.iter()
			.map(|entry| entry.value().clone())
			.collect()
	}

	/// Get widgets visible to the user
	pub async fn get_visible(
		&self,
		user_permissions: &[String],
	) -> Vec<(Arc<dyn DashboardWidget>, WidgetConfig)> {
		let mut visible = Vec::new();
		for entry in self.widgets.iter() {
			let (widget, config) = entry.value();
			if widget.is_visible(user_permissions).await {
				visible.push((widget.clone(), config.clone()));
			}
		}
		visible
	}

	/// Load data for all widgets concurrently
	///
	/// This method loads data for multiple widgets in parallel, which is significantly
	/// faster than loading them sequentially when dealing with many widgets.
	pub async fn load_all_data(
		&self,
		user_permissions: &[String],
	) -> Vec<(String, AdminResult<serde_json::Value>)> {
		let visible_widgets = self.get_visible(user_permissions).await;

		// Use futures::future::join_all for concurrent loading
		let load_futures: Vec<_> = visible_widgets
			.into_iter()
			.map(|(widget, config)| async move {
				let data = widget.load_data().await;
				(config.id, data)
			})
			.collect();

		futures::future::join_all(load_futures).await
	}

	/// Load data for widgets at a specific position concurrently
	pub async fn load_position_data(
		&self,
		position: WidgetPosition,
		user_permissions: &[String],
	) -> Vec<(String, AdminResult<serde_json::Value>)> {
		let widgets = self.get_by_position(position);

		// Filter by visibility
		let mut visible_widgets = Vec::new();
		for (widget, config) in widgets {
			if widget.is_visible(user_permissions).await {
				visible_widgets.push((widget, config));
			}
		}

		// Load data concurrently
		let data_futures: Vec<_> = visible_widgets
			.into_iter()
			.map(|(widget, config)| async move {
				let data = widget.load_data().await;
				(config.id, data)
			})
			.collect();

		futures::future::join_all(data_futures).await
	}
}

impl Default for WidgetRegistry {
	fn default() -> Self {
		Self::new()
	}
}

/// Statistic widget for displaying a single numeric value
pub struct StatWidget {
	title: String,
	icon: Option<String>,
	position: WidgetPosition,
	value_fn: Arc<dyn Fn() -> futures::future::BoxFuture<'static, AdminResult<i64>> + Send + Sync>,
}

impl StatWidget {
	/// Create a new stat widget
	pub fn new<F, Fut>(title: impl Into<String>, position: WidgetPosition, value_fn: F) -> Self
	where
		F: Fn() -> Fut + Send + Sync + 'static,
		Fut: futures::Future<Output = AdminResult<i64>> + Send + 'static,
	{
		Self {
			title: title.into(),
			icon: None,
			position,
			value_fn: Arc::new(move || Box::pin(value_fn())),
		}
	}

	/// Set the icon
	pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
		self.icon = Some(icon.into());
		self
	}
}

#[async_trait]
impl DashboardWidget for StatWidget {
	fn title(&self) -> &str {
		&self.title
	}

	fn icon(&self) -> Option<&str> {
		self.icon.as_deref()
	}

	fn position(&self) -> WidgetPosition {
		self.position
	}

	async fn load_data(&self) -> AdminResult<serde_json::Value> {
		let value = (self.value_fn)().await?;
		Ok(serde_json::json!({ "value": value }))
	}

	async fn render(&self, _context: &WidgetContext) -> AdminResult<String> {
		let data = self.load_data().await?;
		let value = data["value"].as_i64().unwrap_or(0);

		let icon_html = self
			.icon
			.as_ref()
			.map(|i| format!("<i class=\"{}\" aria-hidden=\"true\"></i> ", i))
			.unwrap_or_default();

		Ok(format!(
			r#"<div class="stat-widget">
  <div class="stat-title">{}{}</div>
  <div class="stat-value">{}</div>
</div>"#,
			icon_html, self.title, value
		))
	}
}

/// Chart widget for displaying chart data
pub struct ChartWidget {
	title: String,
	icon: Option<String>,
	position: WidgetPosition,
	chart_type: ChartType,
	data_fn:
		Arc<dyn Fn() -> futures::future::BoxFuture<'static, AdminResult<ChartData>> + Send + Sync>,
}

impl ChartWidget {
	/// Create a new chart widget
	pub fn new<F, Fut>(
		title: impl Into<String>,
		position: WidgetPosition,
		chart_type: ChartType,
		data_fn: F,
	) -> Self
	where
		F: Fn() -> Fut + Send + Sync + 'static,
		Fut: futures::Future<Output = AdminResult<ChartData>> + Send + 'static,
	{
		Self {
			title: title.into(),
			icon: None,
			position,
			chart_type,
			data_fn: Arc::new(move || Box::pin(data_fn())),
		}
	}

	/// Set the icon
	pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
		self.icon = Some(icon.into());
		self
	}
}

#[async_trait]
impl DashboardWidget for ChartWidget {
	fn title(&self) -> &str {
		&self.title
	}

	fn icon(&self) -> Option<&str> {
		self.icon.as_deref()
	}

	fn position(&self) -> WidgetPosition {
		self.position
	}

	fn size(&self) -> (u32, u32) {
		(2, 1)
	}

	async fn load_data(&self) -> AdminResult<serde_json::Value> {
		let data = (self.data_fn)().await?;
		Ok(serde_json::to_value(&data).unwrap())
	}

	async fn render(&self, _context: &WidgetContext) -> AdminResult<String> {
		let data = self.load_data().await?;
		let data_json = serde_json::to_string(&data).unwrap();

		let icon_html = self
			.icon
			.as_ref()
			.map(|i| format!("<i class=\"{}\" aria-hidden=\"true\"></i> ", i))
			.unwrap_or_default();

		Ok(format!(
			r#"<div class="chart-widget">
  <div class="chart-title">{}{}</div>
  <div class="chart-container" data-chart-type="{}" data-chart-data='{}'></div>
</div>"#,
			icon_html,
			self.title,
			match self.chart_type {
				ChartType::Line => "line",
				ChartType::Bar => "bar",
				ChartType::Pie => "pie",
			},
			data_json
		))
	}
}

/// Chart type
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ChartType {
	Line,
	Bar,
	Pie,
}

/// Chart data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartData {
	pub labels: Vec<String>,
	pub datasets: Vec<ChartDataset>,
}

/// Chart dataset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartDataset {
	pub label: String,
	pub data: Vec<f64>,
	pub background_color: Option<Vec<String>>,
	pub border_color: Option<Vec<String>>,
}

/// Recent activity widget
pub struct RecentActivityWidget {
	title: String,
	icon: Option<String>,
	position: WidgetPosition,
	max_items: usize,
	activities_fn: Arc<
		dyn Fn() -> futures::future::BoxFuture<'static, AdminResult<Vec<Activity>>> + Send + Sync,
	>,
}

impl RecentActivityWidget {
	/// Create a new recent activity widget
	pub fn new<F, Fut>(
		title: impl Into<String>,
		position: WidgetPosition,
		max_items: usize,
		activities_fn: F,
	) -> Self
	where
		F: Fn() -> Fut + Send + Sync + 'static,
		Fut: futures::Future<Output = AdminResult<Vec<Activity>>> + Send + 'static,
	{
		Self {
			title: title.into(),
			icon: None,
			position,
			max_items,
			activities_fn: Arc::new(move || Box::pin(activities_fn())),
		}
	}

	/// Set the icon
	pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
		self.icon = Some(icon.into());
		self
	}
}

#[async_trait]
impl DashboardWidget for RecentActivityWidget {
	fn title(&self) -> &str {
		&self.title
	}

	fn icon(&self) -> Option<&str> {
		self.icon.as_deref()
	}

	fn position(&self) -> WidgetPosition {
		self.position
	}

	async fn load_data(&self) -> AdminResult<serde_json::Value> {
		let mut activities = (self.activities_fn)().await?;
		activities.truncate(self.max_items);
		Ok(serde_json::to_value(&activities).unwrap())
	}

	async fn render(&self, _context: &WidgetContext) -> AdminResult<String> {
		let data = self.load_data().await?;
		let activities: Vec<Activity> = serde_json::from_value(data).unwrap();

		let icon_html = self
			.icon
			.as_ref()
			.map(|i| format!("<i class=\"{}\" aria-hidden=\"true\"></i> ", i))
			.unwrap_or_default();

		let items_html = activities
			.iter()
			.map(|a| {
				format!(
					r#"<li class="activity-item">
    <div class="activity-user">{}</div>
    <div class="activity-action">{}</div>
    <div class="activity-time">{}</div>
  </li>"#,
					a.user, a.action, a.timestamp
				)
			})
			.collect::<Vec<_>>()
			.join("\n");

		Ok(format!(
			r#"<div class="activity-widget">
  <div class="activity-title">{}{}</div>
  <ul class="activity-list">
{}
  </ul>
</div>"#,
			icon_html, self.title, items_html
		))
	}
}

/// Activity entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activity {
	pub user: String,
	pub action: String,
	pub timestamp: String,
}

/// Quick links widget
pub struct QuickLinksWidget {
	title: String,
	icon: Option<String>,
	position: WidgetPosition,
	links: Vec<QuickLink>,
}

impl QuickLinksWidget {
	/// Create a new quick links widget
	pub fn new(title: impl Into<String>, position: WidgetPosition) -> Self {
		Self {
			title: title.into(),
			icon: None,
			position,
			links: Vec::new(),
		}
	}

	/// Add a link
	pub fn add_link(mut self, link: QuickLink) -> Self {
		self.links.push(link);
		self
	}

	/// Set the icon
	pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
		self.icon = Some(icon.into());
		self
	}
}

#[async_trait]
impl DashboardWidget for QuickLinksWidget {
	fn title(&self) -> &str {
		&self.title
	}

	fn icon(&self) -> Option<&str> {
		self.icon.as_deref()
	}

	fn position(&self) -> WidgetPosition {
		self.position
	}

	async fn render(&self, _context: &WidgetContext) -> AdminResult<String> {
		let icon_html = self
			.icon
			.as_ref()
			.map(|i| format!("<i class=\"{}\" aria-hidden=\"true\"></i> ", i))
			.unwrap_or_default();

		let links_html = self
			.links
			.iter()
			.map(|link| {
				let link_icon = link
					.icon
					.as_ref()
					.map(|i| format!("<i class=\"{}\" aria-hidden=\"true\"></i> ", i))
					.unwrap_or_default();
				format!(
					r#"<li><a href="{}" class="quick-link">{}{}</a></li>"#,
					link.url, link_icon, link.label
				)
			})
			.collect::<Vec<_>>()
			.join("\n");

		Ok(format!(
			r#"<div class="quick-links-widget">
  <div class="links-title">{}{}</div>
  <ul class="links-list">
{}
  </ul>
</div>"#,
			icon_html, self.title, links_html
		))
	}
}

/// Quick link entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickLink {
	pub label: String,
	pub url: String,
	pub icon: Option<String>,
}

impl QuickLink {
	/// Create a new quick link
	pub fn new(label: impl Into<String>, url: impl Into<String>) -> Self {
		Self {
			label: label.into(),
			url: url.into(),
			icon: None,
		}
	}

	/// Set the icon
	pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
		self.icon = Some(icon.into());
		self
	}
}

/// Table widget for displaying tabular data
/// Type alias for table data function
pub type TableDataFn = Arc<
	dyn Fn() -> futures::future::BoxFuture<'static, AdminResult<Vec<Vec<String>>>> + Send + Sync,
>;

pub struct TableWidget {
	title: String,
	icon: Option<String>,
	position: WidgetPosition,
	columns: Vec<String>,
	data_fn: TableDataFn,
}

impl TableWidget {
	/// Create a new table widget
	pub fn new<F, Fut>(
		title: impl Into<String>,
		position: WidgetPosition,
		columns: Vec<String>,
		data_fn: F,
	) -> Self
	where
		F: Fn() -> Fut + Send + Sync + 'static,
		Fut: futures::Future<Output = AdminResult<Vec<Vec<String>>>> + Send + 'static,
	{
		Self {
			title: title.into(),
			icon: None,
			position,
			columns,
			data_fn: Arc::new(move || Box::pin(data_fn())),
		}
	}

	/// Set the icon
	pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
		self.icon = Some(icon.into());
		self
	}
}

#[async_trait]
impl DashboardWidget for TableWidget {
	fn title(&self) -> &str {
		&self.title
	}

	fn icon(&self) -> Option<&str> {
		self.icon.as_deref()
	}

	fn position(&self) -> WidgetPosition {
		self.position
	}

	fn size(&self) -> (u32, u32) {
		(2, 1)
	}

	async fn load_data(&self) -> AdminResult<serde_json::Value> {
		let rows = (self.data_fn)().await?;
		Ok(serde_json::json!({ "rows": rows }))
	}

	async fn render(&self, _context: &WidgetContext) -> AdminResult<String> {
		let data = self.load_data().await?;
		let rows: Vec<Vec<String>> = serde_json::from_value(data["rows"].clone()).unwrap();

		let icon_html = self
			.icon
			.as_ref()
			.map(|i| format!("<i class=\"{}\" aria-hidden=\"true\"></i> ", i))
			.unwrap_or_default();

		let header_html = self
			.columns
			.iter()
			.map(|col| format!("<th>{}</th>", col))
			.collect::<Vec<_>>()
			.join("");

		let rows_html = rows
			.iter()
			.map(|row| {
				let cells = row
					.iter()
					.map(|cell| format!("<td>{}</td>", cell))
					.collect::<Vec<_>>()
					.join("");
				format!("<tr>{}</tr>", cells)
			})
			.collect::<Vec<_>>()
			.join("\n");

		Ok(format!(
			r#"<div class="table-widget">
  <div class="table-title">{}{}</div>
  <table class="table">
    <thead>
      <tr>{}</tr>
    </thead>
    <tbody>
{}
    </tbody>
  </table>
</div>"#,
			icon_html, self.title, header_html, rows_html
		))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_widget_position() {
		assert_eq!(WidgetPosition::TopLeft, WidgetPosition::TopLeft);
		assert_ne!(WidgetPosition::TopLeft, WidgetPosition::TopRight);
	}

	#[test]
	fn test_widget_context_new() {
		let ctx = WidgetContext::new();
		assert!(ctx.user.is_none());
		assert!(ctx.extra.is_empty());
	}

	#[test]
	fn test_widget_context_with_user() {
		let user = UserInfo {
			username: "admin".to_string(),
			permissions: vec!["view_dashboard".to_string()],
		};
		let ctx = WidgetContext::new().with_user(user.clone());
		assert_eq!(ctx.user.as_ref().unwrap().username, "admin");
	}

	#[test]
	fn test_widget_config_new() {
		let config = WidgetConfig::new("stat1", WidgetPosition::TopLeft);
		assert_eq!(config.id, "stat1");
		assert_eq!(config.position, WidgetPosition::TopLeft);
		assert_eq!(config.size, (1, 1));
		assert_eq!(config.order, 0);
	}

	#[test]
	fn test_widget_config_with_size() {
		let config = WidgetConfig::new("chart1", WidgetPosition::Center).with_size(2, 1);
		assert_eq!(config.size, (2, 1));
	}

	#[test]
	fn test_widget_config_with_order() {
		let config = WidgetConfig::new("widget1", WidgetPosition::TopLeft).with_order(10);
		assert_eq!(config.order, 10);
	}

	#[test]
	fn test_widget_config_add_class() {
		let config = WidgetConfig::new("widget1", WidgetPosition::TopLeft)
			.add_class("custom-class")
			.add_class("another-class");
		assert_eq!(config.css_classes.len(), 2);
		assert!(config.css_classes.contains(&"custom-class".to_string()));
	}

	#[test]
	fn test_widget_config_add_style() {
		let config = WidgetConfig::new("widget1", WidgetPosition::TopLeft)
			.add_style("color", "red")
			.add_style("font-size", "16px");
		assert_eq!(config.style.len(), 2);
		assert_eq!(config.style.get("color"), Some(&"red".to_string()));
	}

	#[test]
	fn test_widget_registry_new() {
		let registry = WidgetRegistry::new();
		assert_eq!(registry.widgets.len(), 0);
	}

	#[tokio::test]
	async fn test_widget_registry_register() {
		let registry = WidgetRegistry::new();
		let widget = Arc::new(QuickLinksWidget::new("Links", WidgetPosition::TopLeft));
		let config = WidgetConfig::new("links1", WidgetPosition::TopLeft);

		let result = registry.register(widget, config);
		assert!(result.is_ok());
		assert_eq!(registry.widgets.len(), 1);
	}

	#[tokio::test]
	async fn test_widget_registry_register_duplicate() {
		let registry = WidgetRegistry::new();
		let widget1 = Arc::new(QuickLinksWidget::new("Links", WidgetPosition::TopLeft));
		let widget2 = Arc::new(QuickLinksWidget::new("Links2", WidgetPosition::TopRight));
		let config1 = WidgetConfig::new("links1", WidgetPosition::TopLeft);
		let config2 = WidgetConfig::new("links1", WidgetPosition::TopRight);

		registry.register(widget1, config1).unwrap();
		let result = registry.register(widget2, config2);
		assert!(result.is_err());
	}

	#[tokio::test]
	async fn test_widget_registry_unregister() {
		let registry = WidgetRegistry::new();
		let widget = Arc::new(QuickLinksWidget::new("Links", WidgetPosition::TopLeft));
		let config = WidgetConfig::new("links1", WidgetPosition::TopLeft);

		registry.register(widget, config).unwrap();
		assert_eq!(registry.widgets.len(), 1);

		let result = registry.unregister("links1");
		assert!(result.is_ok());
		assert_eq!(registry.widgets.len(), 0);
	}

	#[tokio::test]
	async fn test_widget_registry_get() {
		let registry = WidgetRegistry::new();
		let widget = Arc::new(QuickLinksWidget::new("Links", WidgetPosition::TopLeft));
		let config = WidgetConfig::new("links1", WidgetPosition::TopLeft);

		registry.register(widget, config).unwrap();

		let result = registry.get("links1");
		assert!(result.is_some());

		let result = registry.get("nonexistent");
		assert!(result.is_none());
	}

	#[tokio::test]
	async fn test_widget_registry_get_by_position() {
		let registry = WidgetRegistry::new();

		let widget1 = Arc::new(QuickLinksWidget::new("Links1", WidgetPosition::TopLeft));
		let config1 = WidgetConfig::new("links1", WidgetPosition::TopLeft).with_order(2);

		let widget2 = Arc::new(QuickLinksWidget::new("Links2", WidgetPosition::TopLeft));
		let config2 = WidgetConfig::new("links2", WidgetPosition::TopLeft).with_order(1);

		let widget3 = Arc::new(QuickLinksWidget::new("Links3", WidgetPosition::TopRight));
		let config3 = WidgetConfig::new("links3", WidgetPosition::TopRight);

		registry.register(widget1, config1).unwrap();
		registry.register(widget2, config2).unwrap();
		registry.register(widget3, config3).unwrap();

		let top_left = registry.get_by_position(WidgetPosition::TopLeft);
		assert_eq!(top_left.len(), 2);
		// Should be sorted by order
		assert_eq!(top_left[0].1.id, "links2");
		assert_eq!(top_left[1].1.id, "links1");

		let top_right = registry.get_by_position(WidgetPosition::TopRight);
		assert_eq!(top_right.len(), 1);
	}

	#[tokio::test]
	async fn test_quick_links_widget_new() {
		let widget = QuickLinksWidget::new("Quick Links", WidgetPosition::TopLeft);
		assert_eq!(widget.title(), "Quick Links");
		assert_eq!(widget.position(), WidgetPosition::TopLeft);
		assert!(widget.links.is_empty());
	}

	#[tokio::test]
	async fn test_quick_links_widget_add_link() {
		let widget = QuickLinksWidget::new("Quick Links", WidgetPosition::TopLeft)
			.add_link(QuickLink::new("Home", "/"))
			.add_link(QuickLink::new("Users", "/users"));

		assert_eq!(widget.links.len(), 2);
		assert_eq!(widget.links[0].label, "Home");
		assert_eq!(widget.links[1].url, "/users");
	}

	#[tokio::test]
	async fn test_quick_links_widget_render() {
		let widget = QuickLinksWidget::new("Quick Links", WidgetPosition::TopLeft)
			.add_link(QuickLink::new("Home", "/"))
			.add_link(QuickLink::new("Users", "/users"));

		let ctx = WidgetContext::new();
		let html = widget.render(&ctx).await.unwrap();

		assert!(html.contains("Quick Links"));
		assert!(html.contains("Home"));
		assert!(html.contains("Users"));
		assert!(html.contains("href=\"/\""));
		assert!(html.contains("href=\"/users\""));
	}

	#[tokio::test]
	async fn test_quick_link_with_icon() {
		let link = QuickLink::new("Dashboard", "/dashboard").with_icon("fa fa-dashboard");

		assert_eq!(link.icon, Some("fa fa-dashboard".to_string()));
	}

	#[tokio::test]
	async fn test_stat_widget_render() {
		let widget = StatWidget::new("Total Users", WidgetPosition::TopLeft, || async { Ok(42) });

		let ctx = WidgetContext::new();
		let html = widget.render(&ctx).await.unwrap();

		assert!(html.contains("Total Users"));
		assert!(html.contains("42"));
	}

	#[tokio::test]
	async fn test_stat_widget_with_icon() {
		let widget = StatWidget::new("Total Users", WidgetPosition::TopLeft, || async { Ok(100) })
			.with_icon("fa fa-users");

		assert_eq!(widget.icon(), Some("fa fa-users"));

		let ctx = WidgetContext::new();
		let html = widget.render(&ctx).await.unwrap();
		assert!(html.contains("fa fa-users"));
	}

	#[tokio::test]
	async fn test_chart_widget_render() {
		let widget = ChartWidget::new(
			"Sales Chart",
			WidgetPosition::Center,
			ChartType::Line,
			|| async {
				Ok(ChartData {
					labels: vec!["Jan".to_string(), "Feb".to_string()],
					datasets: vec![ChartDataset {
						label: "Sales".to_string(),
						data: vec![100.0, 200.0],
						background_color: None,
						border_color: None,
					}],
				})
			},
		);

		assert_eq!(widget.size(), (2, 1));

		let ctx = WidgetContext::new();
		let html = widget.render(&ctx).await.unwrap();

		assert!(html.contains("Sales Chart"));
		assert!(html.contains("data-chart-type=\"line\""));
	}

	#[tokio::test]
	async fn test_recent_activity_widget_render() {
		let widget =
			RecentActivityWidget::new("Recent Actions", WidgetPosition::BottomLeft, 5, || async {
				Ok(vec![
					Activity {
						user: "admin".to_string(),
						action: "Created user".to_string(),
						timestamp: "2025-01-01".to_string(),
					},
					Activity {
						user: "editor".to_string(),
						action: "Updated post".to_string(),
						timestamp: "2025-01-02".to_string(),
					},
				])
			});

		let ctx = WidgetContext::new();
		let html = widget.render(&ctx).await.unwrap();

		assert!(html.contains("Recent Actions"));
		assert!(html.contains("admin"));
		assert!(html.contains("Created user"));
		assert!(html.contains("editor"));
		assert!(html.contains("Updated post"));
	}

	#[tokio::test]
	async fn test_table_widget_render() {
		let widget = TableWidget::new(
			"Recent Orders",
			WidgetPosition::Center,
			vec![
				"Order ID".to_string(),
				"Customer".to_string(),
				"Total".to_string(),
			],
			|| async {
				Ok(vec![
					vec!["001".to_string(), "Alice".to_string(), "$100".to_string()],
					vec!["002".to_string(), "Bob".to_string(), "$200".to_string()],
				])
			},
		);

		assert_eq!(widget.size(), (2, 1));

		let ctx = WidgetContext::new();
		let html = widget.render(&ctx).await.unwrap();

		assert!(html.contains("Recent Orders"));
		assert!(html.contains("Order ID"));
		assert!(html.contains("Customer"));
		assert!(html.contains("Alice"));
		assert!(html.contains("$100"));
		assert!(html.contains("Bob"));
	}

	#[test]
	fn test_chart_type_serialization() {
		let chart_type = ChartType::Line;
		let json = serde_json::to_string(&chart_type).unwrap();
		assert!(json.contains("Line"));

		let deserialized: ChartType = serde_json::from_str(&json).unwrap();
		assert!(matches!(deserialized, ChartType::Line));
	}
}
