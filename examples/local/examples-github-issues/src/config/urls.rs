//! URL configuration for examples-github-issues project
//!
//! This module configures the unified GraphQL schema and URL patterns.

use async_graphql::{
	MergedObject, MergedSubscription, Schema,
	http::{GraphQLPlaygroundConfig, playground_source},
};
use reinhardt::routes;
use reinhardt::{JwtAuth, Request, Response, StatusCode, UnifiedRouter, ViewResult};

use crate::apps::auth::views::{AuthMutation, AuthQuery, UserStorage};
use crate::apps::issues::views::{
	IssueEventBroadcaster, IssueMutation, IssueQuery, IssueStorage, IssueSubscription,
};
use crate::apps::projects::views::{
	ProjectMemberStorage, ProjectMutation, ProjectQuery, ProjectStorage,
};

use super::views;

/// Merged Query type combining all app queries
#[derive(MergedObject, Default)]
pub struct Query(AuthQuery, IssueQuery, ProjectQuery);

/// Merged Mutation type combining all app mutations
#[derive(MergedObject, Default)]
pub struct Mutation(AuthMutation, IssueMutation, ProjectMutation);

/// Merged Subscription type for real-time updates
#[derive(MergedSubscription, Default)]
pub struct Subscription(IssueSubscription);

/// GraphQL schema type alias
pub type AppSchema = Schema<Query, Mutation, Subscription>;

/// Create the unified GraphQL schema with all context data
pub fn create_schema() -> AppSchema {
	let user_storage = UserStorage::new();
	let issue_storage = IssueStorage::new();
	let project_storage = ProjectStorage::new();
	let member_storage = ProjectMemberStorage::new();
	let broadcaster = IssueEventBroadcaster::new();

	// JWT secret should be loaded from settings in production
	let jwt_auth = JwtAuth::new(b"your-secret-key-change-in-production");

	Schema::build(
		Query::default(),
		Mutation::default(),
		Subscription::default(),
	)
	.data(user_storage)
	.data(issue_storage)
	.data(project_storage)
	.data(member_storage)
	.data(broadcaster)
	.data(jwt_auth)
	.finish()
}

/// GraphQL query/mutation handler
pub async fn graphql_handler(req: Request) -> ViewResult<Response> {
	// Get schema from app state (in real app, this would be injected)
	// For now, create a new schema per request (not ideal for production)
	let schema = create_schema();

	// Parse request body as GraphQL request
	let graphql_request: async_graphql::Request = req
		.json()
		.map_err(|e| format!("Failed to parse GraphQL request: {}", e))?;

	// Extract JWT token if present and add claims to context
	let graphql_request = if let Some(auth_header) = req
		.headers
		.get("authorization")
		.and_then(|h| h.to_str().ok())
	{
		if let Some(token) = auth_header.strip_prefix("Bearer ") {
			let jwt_auth = JwtAuth::new(b"your-secret-key-change-in-production");
			if let Ok(claims) = jwt_auth.verify_token(token) {
				graphql_request.data(claims)
			} else {
				graphql_request
			}
		} else {
			graphql_request
		}
	} else {
		graphql_request
	};

	// Execute GraphQL request
	let response = schema.execute(graphql_request).await;

	// Serialize response
	let json = serde_json::to_string(&response)
		.map_err(|e| format!("Failed to serialize GraphQL response: {}", e))?;

	Ok(Response::new(StatusCode::OK)
		.with_header("Content-Type", "application/json")
		.with_body(json))
}

/// GraphQL Playground handler (development only)
pub async fn graphql_playground(_req: Request) -> ViewResult<Response> {
	let html = playground_source(GraphQLPlaygroundConfig::new("/graphql"));

	Ok(Response::new(StatusCode::OK)
		.with_header("Content-Type", "text/html")
		.with_body(html))
}

/// Build URL patterns for the application
#[routes]
pub fn routes() -> UnifiedRouter {
	UnifiedRouter::new()
		.endpoint(views::health_check)
		.function("/graphql", reinhardt::Method::POST, graphql_handler)
		.function("/graphql", reinhardt::Method::GET, graphql_playground)
}
