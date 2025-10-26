//! Application configuration and registry for Reinhardt
//!
//! ## Planned Features
//! - Full reverse relation building (requires ORM relationship system integration)
//! - Migration detection and management (requires reinhardt-migrations crate)

pub mod apps;
pub mod builder;
pub mod discovery;
pub mod hooks;
pub mod registry;
pub mod signals;

// Re-export from reinhardt-http
pub use reinhardt_http::{Request, Response, StreamBody, StreamingResponse};

// Re-export from reinhardt-settings
pub use reinhardt_settings::{DatabaseConfig, MiddlewareConfig, Settings, TemplateConfig};

// Re-export from reinhardt-exception
pub use reinhardt_exception::{Error, Result};

// Re-export from reinhardt-server
pub use reinhardt_server::{serve, HttpServer};

// Re-export from reinhardt-types
pub use reinhardt_types::{Handler, Middleware, MiddlewareChain};

// Re-export from apps module
pub use apps::{get_apps, init_apps, init_apps_checked, AppConfig, AppError, AppResult, Apps};

// Re-export from builder module
pub use builder::{
    Application, ApplicationBuilder, ApplicationDatabaseConfig, BuildError, BuildResult,
    RouteConfig,
};

// Re-export from registry module
pub use registry::{
    clear_model_cache, clear_relationship_cache, clear_reverse_relations, find_model,
    get_models_for_app, get_registered_models, get_registered_relationships,
    get_relationships_for_model, get_relationships_to_model,
    get_reverse_relations_for_model, register_reverse_relation, ModelMetadata,
    RelationshipMetadata, RelationshipType, ReverseRelationMetadata, ReverseRelationType, MODELS,
    RELATIONSHIPS,
};

// Re-export from discovery module
pub use discovery::{
    build_reverse_relations, create_reverse_relation, discover_all_models, discover_migrations,
    discover_models, MigrationMetadata, RelationMetadata, RelationType,
};

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use hyper::{HeaderMap, Method, Uri, Version};

    #[test]
    fn test_request_query_params() {
        let uri = Uri::from_static("/test?foo=bar&baz=qux");
        let request = Request::new(
            Method::GET,
            uri,
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::new(),
        );

        assert_eq!(request.query_params.get("foo"), Some(&"bar".to_string()));
        assert_eq!(request.query_params.get("baz"), Some(&"qux".to_string()));
    }

    #[test]
    fn test_response_creation() {
        let response = Response::ok();
        assert_eq!(response.status, hyper::StatusCode::OK);

        let response = Response::created();
        assert_eq!(response.status, hyper::StatusCode::CREATED);

        let response = Response::not_found();
        assert_eq!(response.status, hyper::StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_response_with_json_unit() {
        use serde_json::json;

        let data = json!({
            "message": "Hello, world!"
        });

        let response = Response::ok().with_json(&data).unwrap();

        let body_str = String::from_utf8(response.body.to_vec()).unwrap();
        assert!(body_str.contains("Hello, world!"));
        assert_eq!(
            response.headers.get(hyper::header::CONTENT_TYPE).unwrap(),
            "application/json"
        );
    }

    #[test]
    fn test_error_status_codes() {
        assert_eq!(Error::NotFound("test".into()).status_code(), 404);
        assert_eq!(Error::Authentication("test".into()).status_code(), 401);
        assert_eq!(Error::Authorization("test".into()).status_code(), 403);
        assert_eq!(Error::Validation("test".into()).status_code(), 400);
        assert_eq!(Error::Internal("test".into()).status_code(), 500);
    }
}
