//! Tera-based Runtime Template Renderer
//!
//! This module provides a runtime template renderer using the Tera template engine.
//! Tera is a powerful template engine inspired by Jinja2 and Django templates.
//!
//! # Performance Characteristics
//!
//! - **Time Complexity**: O(n) - Runtime template parsing and rendering
//! - **Space Complexity**: O(n) - Templates cached in memory
//! - **Performance**: Slower than compile-time templates but more flexible
//!
//! # Use Cases
//!
//! Use Tera renderer for:
//! - View templates (HTML pages)
//! - Email templates
//! - Dynamic response templates
//! - User-provided templates
//! - Templates loaded at runtime
//! - Templates stored in database
//!
//! # Examples
//!
//! ```rust
//! use reinhardt_renderers::TeraRenderer;
//! use serde_json::json;
//!
//! let renderer = TeraRenderer::new();
//! let context = json!({
//!     "name": "Alice",
//!     "email": "alice@example.com",
//!     "age": 25
//! });
//!
//! let html = renderer.render_template("user.tpl", &context)
//!     .expect("Failed to render template");
//! ```

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use tera::{Context, Tera};

// グローバル Tera インスタンス（遅延初期化）
static TERA: Lazy<Tera> = Lazy::new(|| {
	let mut tera = Tera::default();

	// テンプレートを include_str! で読み込み、登録
	tera.add_raw_template("user.tpl", include_str!("../templates/user.tpl"))
		.expect("Failed to add user.tpl template");

	tera.add_raw_template("user_list.tpl", include_str!("../templates/user_list.tpl"))
		.expect("Failed to add user_list.tpl template");

	tera.add_raw_template("posts.tpl", include_str!("../templates/posts.tpl"))
		.expect("Failed to add posts.tpl template");

	tera.add_raw_template(
		"documentation.tpl",
		include_str!("../templates/documentation.tpl"),
	)
	.expect("Failed to add documentation.tpl template");

	tera.add_raw_template("admin.tpl", include_str!("../templates/admin.tpl"))
		.expect("Failed to add admin.tpl template");

	tera
});

/// Tera-based runtime template renderer
///
/// This renderer uses Tera's runtime template compilation for flexible
/// template rendering. Templates are parsed and rendered at runtime,
/// providing maximum flexibility for dynamic content.
///
/// # Runtime Safety
///
/// - Templates are parsed at runtime
/// - Type mismatches are caught during rendering
/// - Template syntax errors are returned as errors
///
/// # Performance
///
/// Performance comparison (measured in microseconds):
///
/// | Variables | Compile-time (Tera) | Runtime (Tera) | Ratio |
/// |-----------|------------------------|----------------|-------|
/// | 10        | 0.01μs                 | 10μs           | 1000x |
/// | 100       | 0.01μs                 | 100μs          | 10000x|
/// | 1000      | 0.01μs                 | 1000μs         | 100000x|
pub struct TeraRenderer;

impl TeraRenderer {
	/// Creates a new TeraRenderer
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::TeraRenderer;
	///
	/// let renderer = TeraRenderer::new();
	/// ```
	pub fn new() -> Self {
		Self
	}

	/// Renders a template with the given context
	///
	/// # Arguments
	///
	/// * `template_name` - Name of the template to render (e.g., "user.tpl")
	/// * `context` - Context data implementing `Serialize`
	///
	/// # Returns
	///
	/// - `Ok(String)` - Rendered HTML string
	/// - `Err(String)` - Error message if rendering fails
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::TeraRenderer;
	/// use serde_json::json;
	///
	/// let renderer = TeraRenderer::new();
	/// let context = json!({
	///     "name": "Alice",
	///     "email": "alice@example.com",
	///     "age": 25
	/// });
	///
	/// let html = renderer.render_template("user.tpl", &context)
	///     .expect("Failed to render template");
	/// ```
	pub fn render_template<T: Serialize>(
		&self,
		template_name: &str,
		context: &T,
	) -> Result<String, String> {
		let ctx = Context::from_serialize(context)
			.map_err(|e| format!("Failed to create context: {}", e))?;

		TERA.render(template_name, &ctx)
			.map_err(|e| format!("Tera rendering error: {}", e))
	}

	/// Renders a template and returns the result with error context
	///
	/// This is a convenience method that provides more detailed error information.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::TeraRenderer;
	/// use serde_json::json;
	///
	/// let renderer = TeraRenderer::new();
	/// let context = json!({
	///     "name": "Alice",
	///     "email": "alice@example.com",
	///     "age": 25
	/// });
	///
	/// match renderer.render_with_context("user.tpl", &context, "user profile") {
	///     Ok(html) => println!("{}", html),
	///     Err(e) => eprintln!("Failed to render user profile: {}", e),
	/// }
	/// ```
	pub fn render_with_context<T: Serialize>(
		&self,
		template_name: &str,
		context: &T,
		context_name: &str,
	) -> Result<String, String> {
		self.render_template(template_name, context)
			.map_err(|e| format!("Failed to render {}: {}", context_name, e))
	}
}

impl Default for TeraRenderer {
	fn default() -> Self {
		Self::new()
	}
}

/// User template data
///
/// This struct demonstrates how to create template data for Tera templates.
///
/// # Template File
///
/// The template file `templates/user.tpl` is used for rendering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserTemplate {
	pub name: String,
	pub email: String,
	pub age: u32,
}

impl UserTemplate {
	/// Creates a new UserTemplate
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::UserTemplate;
	///
	/// let template = UserTemplate::new(
	///     "Alice".to_string(),
	///     "alice@example.com".to_string(),
	///     25
	/// );
	/// ```
	pub fn new(name: String, email: String, age: u32) -> Self {
		Self { name, email, age }
	}

	/// Renders the user template
	///
	/// This is a convenience method that creates a renderer and renders the template.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::UserTemplate;
	///
	/// let template = UserTemplate::new(
	///     "Alice".to_string(),
	///     "alice@example.com".to_string(),
	///     25
	/// );
	///
	/// let html = template.render_user().expect("Failed to render");
	/// assert!(html.contains("Alice"));
	/// assert!(html.contains("alice@example.com"));
	/// ```
	pub fn render_user(&self) -> Result<String, String> {
		let renderer = TeraRenderer::new();
		renderer.render_template("user.tpl", self)
	}
}

/// User data for list template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserData {
	pub name: String,
	pub email: String,
}

impl UserData {
	/// Creates a new UserData
	pub fn new(name: impl Into<String>, email: impl Into<String>) -> Self {
		Self {
			name: name.into(),
			email: email.into(),
		}
	}
}

impl Display for UserData {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{} ({})", self.name, self.email)
	}
}

/// User list template data
///
/// This template demonstrates rendering lists with Tera.
///
/// # Template File
///
/// The template file `templates/user_list.tpl` is used for rendering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserListTemplate {
	pub users: Vec<UserData>,
	pub title: String,
}

impl UserListTemplate {
	/// Creates a new UserListTemplate
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::{UserListTemplate, UserData};
	///
	/// let users = vec![
	///     UserData::new("Alice", "alice@example.com"),
	///     UserData::new("Bob", "bob@example.com"),
	/// ];
	///
	/// let template = UserListTemplate::new(users, "User Directory".to_string());
	/// ```
	pub fn new(users: Vec<UserData>, title: String) -> Self {
		Self { users, title }
	}

	/// Renders the user list template
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::{UserListTemplate, UserData};
	///
	/// let users = vec![
	///     UserData::new("Alice", "alice@example.com"),
	///     UserData::new("Bob", "bob@example.com"),
	/// ];
	///
	/// let template = UserListTemplate::new(users, "User Directory".to_string());
	/// let html = template.render_list().expect("Failed to render");
	///
	/// assert!(html.contains("Alice"));
	/// assert!(html.contains("Bob"));
	/// assert!(html.contains("User Directory"));
	/// ```
	pub fn render_list(&self) -> Result<String, String> {
		let renderer = TeraRenderer::new();
		renderer.render_template("user_list.tpl", self)
	}
}

/// Post data for blog/content templates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
	pub id: i64,
	pub title: String,
	pub content: String,
	pub author: String,
}

impl Post {
	/// Creates a new Post
	pub fn new(
		id: i64,
		title: impl Into<String>,
		content: impl Into<String>,
		author: impl Into<String>,
	) -> Self {
		Self {
			id,
			title: title.into(),
			content: content.into(),
			author: author.into(),
		}
	}
}

impl Display for Post {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{} by {}", self.title, self.author)
	}
}

/// Post list template data
///
/// This template demonstrates rendering a list of blog posts.
///
/// # Template File
///
/// The template file `templates/posts.tpl` is used for rendering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostListTemplate {
	pub posts: Vec<Post>,
	pub total: usize,
}

impl PostListTemplate {
	/// Creates a new PostListTemplate
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::{PostListTemplate, Post};
	///
	/// let posts = vec![
	///     Post::new(1, "First Post", "Hello World", "Alice"),
	///     Post::new(2, "Second Post", "Goodbye World", "Bob"),
	/// ];
	///
	/// let template = PostListTemplate::new(posts);
	/// ```
	pub fn new(posts: Vec<Post>) -> Self {
		let total = posts.len();
		Self { posts, total }
	}

	/// Renders the post list template
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::{PostListTemplate, Post};
	///
	/// let posts = vec![
	///     Post::new(1, "First Post", "Hello World", "Alice"),
	///     Post::new(2, "Second Post", "Goodbye World", "Bob"),
	/// ];
	///
	/// let template = PostListTemplate::new(posts);
	/// let html = template.render_posts().expect("Failed to render");
	///
	/// assert!(html.contains("First Post"));
	/// assert!(html.contains("Second Post"));
	/// ```
	pub fn render_posts(&self) -> Result<String, String> {
		let renderer = TeraRenderer::new();
		renderer.render_template("posts.tpl", self)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_tera_renderer_new() {
		let renderer = TeraRenderer::new();
		let _ = renderer;
	}

	#[test]
	fn test_user_template_render() {
		let template = UserTemplate::new("Alice".to_string(), "alice@example.com".to_string(), 25);

		let html = template
			.render_user()
			.expect("Failed to render user template");

		assert!(
			html.starts_with("<!DOCTYPE html>"),
			"Rendered template should start with DOCTYPE declaration, got: {}",
			&html[..100.min(html.len())]
		);

		let name_count = html.matches("Alice").count();
		assert_eq!(
			name_count, 1,
			"User name 'Alice' should appear exactly once in template, found {} times",
			name_count
		);

		let email_count = html.matches("alice@example.com").count();
		assert_eq!(
			email_count, 1,
			"Email should appear exactly once, found {} times",
			email_count
		);

		let age_count = html.matches("25").count();
		assert_eq!(
			age_count, 1,
			"Age should appear exactly once, found {} times",
			age_count
		);

		let adult_count = html.matches("Adult").count();
		assert_eq!(
			adult_count, 1,
			"Adult status should be shown exactly once for age >= 18, found {} times",
			adult_count
		);
	}

	#[test]
	fn test_user_template_minor() {
		let template =
			UserTemplate::new("Charlie".to_string(), "charlie@example.com".to_string(), 16);

		let html = template
			.render_user()
			.expect("Failed to render user template");

		let name_count = html.matches("Charlie").count();
		assert_eq!(
			name_count, 1,
			"User name 'Charlie' should appear exactly once, found {} times",
			name_count
		);

		let age_count = html.matches("16").count();
		assert_eq!(
			age_count, 1,
			"Age should appear exactly once, found {} times",
			age_count
		);

		let minor_count = html.matches("Minor").count();
		assert_eq!(
			minor_count, 1,
			"Minor status should be shown exactly once for age < 18, found {} times",
			minor_count
		);
	}

	#[test]
	fn test_user_list_template_render() {
		let users = vec![
			UserData::new("Alice", "alice@example.com"),
			UserData::new("Bob", "bob@example.com"),
			UserData::new("Charlie", "charlie@example.com"),
		];

		let template = UserListTemplate::new(users, "User Directory".to_string());
		let html = template
			.render_list()
			.expect("Failed to render list template");

		assert!(
			html.starts_with("<!DOCTYPE html>"),
			"Template should start with DOCTYPE, got: {}",
			&html[..50.min(html.len())]
		);

		let title_count = html.matches("User Directory").count();
		assert_eq!(
			title_count, 2,
			"Title should appear twice (in <title> and <h1>), found {} times",
			title_count
		);

		let alice_count = html.matches("Alice").count();
		assert_eq!(
			alice_count, 1,
			"User 'Alice' should appear exactly once, found {} times",
			alice_count
		);

		let alice_email_count = html.matches("alice@example.com").count();
		assert_eq!(
			alice_email_count, 1,
			"Alice's email should appear exactly once, found {} times",
			alice_email_count
		);

		let bob_count = html.matches("Bob").count();
		assert_eq!(
			bob_count, 1,
			"User 'Bob' should appear exactly once, found {} times",
			bob_count
		);

		let bob_email_count = html.matches("bob@example.com").count();
		assert_eq!(
			bob_email_count, 1,
			"Bob's email should appear exactly once, found {} times",
			bob_email_count
		);

		let charlie_count = html.matches("Charlie").count();
		assert_eq!(
			charlie_count, 1,
			"User 'Charlie' should appear exactly once, found {} times",
			charlie_count
		);

		let charlie_email_count = html.matches("charlie@example.com").count();
		assert_eq!(
			charlie_email_count, 1,
			"Charlie's email should appear exactly once, found {} times",
			charlie_email_count
		);
	}

	#[test]
	fn test_user_list_template_empty() {
		let users = vec![];

		let template = UserListTemplate::new(users, "Empty List".to_string());
		let html = template.render_list().expect("Failed to render empty list");

		assert!(
			html.starts_with("<!DOCTYPE html>"),
			"Empty list template should start with DOCTYPE, got: {}",
			&html[..50.min(html.len())]
		);

		let title_count = html.matches("Empty List").count();
		assert_eq!(
			title_count, 2,
			"Title should appear twice, found {} times",
			title_count
		);

		let no_users_count = html.matches("No users found").count();
		assert_eq!(
			no_users_count, 1,
			"No users message should appear exactly once, found {} times",
			no_users_count
		);
	}

	#[test]
	fn test_tera_renderer_render() {
		let renderer = TeraRenderer::new();
		let template =
			UserTemplate::new("Test User".to_string(), "test@example.com".to_string(), 30);

		let html = renderer
			.render_template("user.tpl", &template)
			.expect("Failed to render");

		let name_count = html.matches("Test User").count();
		assert_eq!(
			name_count, 1,
			"User name should appear exactly once, found {} times",
			name_count
		);

		let email_count = html.matches("test@example.com").count();
		assert_eq!(
			email_count, 1,
			"Email should appear exactly once, found {} times",
			email_count
		);

		let age_count = html.matches("30").count();
		assert_eq!(
			age_count, 1,
			"Age should appear exactly once, found {} times",
			age_count
		);
	}

	#[test]
	fn test_tera_renderer_render_with_context() {
		let renderer = TeraRenderer::new();
		let template = UserTemplate::new(
			"Context Test".to_string(),
			"context@example.com".to_string(),
			22,
		);

		let html = renderer
			.render_with_context("user.tpl", &template, "user profile")
			.expect("Failed to render with context");

		let name_count = html.matches("Context Test").count();
		assert_eq!(
			name_count, 1,
			"User name should appear exactly once, found {} times",
			name_count
		);
	}

	#[test]
	fn test_user_data_display() {
		let user = UserData::new("Display Test", "display@example.com");

		let display_string = format!("{}", user);
		assert_eq!(display_string, "Display Test (display@example.com)");
	}

	#[test]
	fn test_user_template_new() {
		let template = UserTemplate::new("New Test".to_string(), "new@example.com".to_string(), 18);

		assert_eq!(template.name, "New Test");
		assert_eq!(template.email, "new@example.com");
		assert_eq!(template.age, 18);
	}

	#[test]
	fn test_user_list_template_new() {
		let users = vec![UserData::new("User1", "user1@example.com")];

		let template = UserListTemplate::new(users.clone(), "Test List".to_string());

		assert_eq!(template.title, "Test List");
		assert_eq!(template.users.len(), 1);
		assert_eq!(template.users[0].name, "User1");
	}

	#[test]
	fn test_user_data_new() {
		let user = UserData::new("Data Test", "data@example.com");

		assert_eq!(user.name, "Data Test");
		assert_eq!(user.email, "data@example.com");
	}

	#[test]
	fn test_post_new() {
		let post = Post::new(1, "Test Post", "Test Content", "Test Author");

		assert_eq!(post.id, 1);
		assert_eq!(post.title, "Test Post");
		assert_eq!(post.content, "Test Content");
		assert_eq!(post.author, "Test Author");
	}

	#[test]
	fn test_post_display() {
		let post = Post::new(1, "My Post", "Content", "Alice");

		let display_string = format!("{}", post);
		assert_eq!(display_string, "My Post by Alice");
	}

	#[test]
	fn test_post_list_template_new() {
		let posts = vec![
			Post::new(1, "First", "Content 1", "Alice"),
			Post::new(2, "Second", "Content 2", "Bob"),
		];

		let template = PostListTemplate::new(posts.clone());

		assert_eq!(template.total, 2);
		assert_eq!(template.posts.len(), 2);
		assert_eq!(template.posts[0].title, "First");
		assert_eq!(template.posts[1].title, "Second");
	}

	#[test]
	fn test_post_list_template_render() {
		let posts = vec![
			Post::new(1, "First Post", "Hello World", "Alice"),
			Post::new(2, "Second Post", "Goodbye World", "Bob"),
		];

		let template = PostListTemplate::new(posts);
		let html = template.render_posts().expect("Failed to render posts");

		assert!(
			html.starts_with("<!DOCTYPE html>"),
			"Template should start with DOCTYPE, got: {}",
			&html[..50.min(html.len())]
		);

		let title_with_count = html.matches("Posts (2)").count();
		assert_eq!(
			title_with_count, 1,
			"Title with count should appear exactly once in <title>, found {} times",
			title_with_count
		);

		let all_posts_count = html.matches("All Posts").count();
		assert_eq!(
			all_posts_count, 1,
			"All Posts heading should appear exactly once, found {} times",
			all_posts_count
		);

		let first_post_count = html.matches("First Post").count();
		assert_eq!(
			first_post_count, 1,
			"First post title should appear exactly once, found {} times",
			first_post_count
		);

		let hello_world_count = html.matches("Hello World").count();
		assert_eq!(
			hello_world_count, 1,
			"First post content should appear exactly once, found {} times",
			hello_world_count
		);

		let alice_count = html.matches("Alice").count();
		assert_eq!(
			alice_count, 1,
			"First post author should appear exactly once, found {} times",
			alice_count
		);

		let second_post_count = html.matches("Second Post").count();
		assert_eq!(
			second_post_count, 1,
			"Second post title should appear exactly once, found {} times",
			second_post_count
		);

		let goodbye_world_count = html.matches("Goodbye World").count();
		assert_eq!(
			goodbye_world_count, 1,
			"Second post content should appear exactly once, found {} times",
			goodbye_world_count
		);

		let bob_count = html.matches("Bob").count();
		assert_eq!(
			bob_count, 1,
			"Second post author should appear exactly once, found {} times",
			bob_count
		);
	}

	#[test]
	fn test_post_list_template_empty() {
		let posts = vec![];

		let template = PostListTemplate::new(posts);
		let html = template
			.render_posts()
			.expect("Failed to render empty posts");

		assert!(
			html.starts_with("<!DOCTYPE html>"),
			"Empty posts template should start with DOCTYPE, got: {}",
			&html[..50.min(html.len())]
		);

		let title_with_count = html.matches("Posts (0)").count();
		assert_eq!(
			title_with_count, 1,
			"Title with zero count should appear exactly once, found {} times",
			title_with_count
		);

		let no_posts_count = html.matches("No posts available").count();
		assert_eq!(
			no_posts_count, 1,
			"No posts message should appear exactly once, found {} times",
			no_posts_count
		);
	}
}
