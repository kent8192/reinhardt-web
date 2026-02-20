//! GraphQL developer tools
//!
//! This module provides developer tools for GraphQL APIs:
//! - GraphiQL interactive query explorer
//! - SDL (Schema Definition Language) export

use async_graphql::Schema;
use async_graphql::http::GraphiQLSource;

/// Generate GraphiQL HTML page for interactive GraphQL exploration
///
/// # Arguments
///
/// * `endpoint` - The GraphQL endpoint URL (e.g., "/graphql")
///
/// # Returns
///
/// HTML string for the GraphiQL interface
///
/// # Example
///
/// ```rust,ignore
/// use reinhardt_graphql::playground::graphiql_html;
///
/// let html = graphiql_html("/graphql");
/// // Use in axum/warp handler: Html(graphiql_html("/graphql"))
/// ```
pub fn graphiql_html(endpoint: &str) -> String {
	GraphiQLSource::build().endpoint(endpoint).finish()
}

/// Generate GraphiQL HTML with custom title
///
/// # Arguments
///
/// * `endpoint` - The GraphQL endpoint URL
/// * `title` - Custom page title
pub fn graphiql_html_with_title(endpoint: &str, title: &str) -> String {
	let html = GraphiQLSource::build().endpoint(endpoint).finish();

	// Replace default title with custom title
	html.replace(
		"<title>GraphiQL</title>",
		&format!("<title>{}</title>", title),
	)
}

/// Export GraphQL schema as SDL (Schema Definition Language)
///
/// # Arguments
///
/// * `schema` - The GraphQL schema to export
///
/// # Returns
///
/// SDL string representation of the schema
///
/// # Example
///
/// ```rust,ignore
/// use reinhardt_graphql::playground::export_sdl;
/// use async_graphql::Schema;
///
/// let sdl = export_sdl(&schema);
/// println!("{}", sdl);
/// ```
pub fn export_sdl<Q, M, S>(schema: &Schema<Q, M, S>) -> String
where
	Q: async_graphql::ObjectType + 'static,
	M: async_graphql::ObjectType + 'static,
	S: async_graphql::SubscriptionType + 'static,
{
	schema.sdl()
}

#[cfg(test)]
mod tests {
	use rstest::rstest;

	#[rstest]
	fn test_graphiql_html_contains_endpoint() {
		// Arrange
		let endpoint = "/graphql";

		// Act
		let html = super::graphiql_html(endpoint);

		// Assert
		assert!(html.contains(endpoint));
		assert!(html.contains("GraphiQL"));
	}

	#[rstest]
	fn test_graphiql_html_with_custom_title() {
		// Arrange
		let endpoint = "/api/graphql";
		let title = "My API Explorer";

		// Act
		let html = super::graphiql_html_with_title(endpoint, title);

		// Assert
		assert!(html.contains(endpoint));
		assert!(html.contains(&format!("<title>{}</title>", title)));
	}
}
