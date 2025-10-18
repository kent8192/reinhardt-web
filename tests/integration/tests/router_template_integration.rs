// Router and Template integration tests
// Inspired by Django's template rendering in views and FastAPI's Jinja2Templates

use askama::Template;
use async_trait::async_trait;
use bytes::Bytes;
use hyper::{HeaderMap, Method, StatusCode, Uri, Version};
use reinhardt_exception::Result;
use reinhardt_templates::{TemplateLoader, TemplateResult};
use reinhardt_types::{Handler, Request, Response};
use std::sync::Arc;

// Example template structure using Askama
#[derive(Template, Clone)]
#[template(source = "Hello {{ name }}!", ext = "txt")]
struct HelloTemplate {
    name: String,
}

#[derive(Template, Clone)]
#[template(source = "User: {{ username }}, Email: {{ email }}", ext = "txt")]
struct UserTemplate {
    username: String,
    email: String,
}

#[derive(Template, Clone)]
#[template(
    source = "{% for item in items %}{{ item }}{% if !loop.last %}, {% endif %}{% endfor %}",
    ext = "txt"
)]
struct ListTemplate {
    items: Vec<String>,
}

// Template-based handler
#[derive(Clone)]
struct TemplateHandler<T: Template + Clone> {
    template: T,
}

impl<T: Template + Clone> TemplateHandler<T> {
    fn new(template: T) -> Self {
        Self { template }
    }
}

#[async_trait]
impl<T: Template + Clone + Send + Sync> Handler for TemplateHandler<T> {
    async fn handle(&self, _request: Request) -> Result<Response> {
        let rendered = self
            .template
            .render()
            .map_err(|e| reinhardt_exception::Error::Internal(e.to_string()))?;

        Ok(Response::ok()
            .with_body(Bytes::from(rendered))
            .with_header("content-type", "text/html; charset=utf-8"))
    }
}

// Dynamic template handler using TemplateLoader
struct DynamicTemplateHandler {
    loader: Arc<TemplateLoader>,
    template_name: String,
}

impl DynamicTemplateHandler {
    fn new(loader: Arc<TemplateLoader>, template_name: impl Into<String>) -> Self {
        Self {
            loader,
            template_name: template_name.into(),
        }
    }
}

#[async_trait]
impl Handler for DynamicTemplateHandler {
    async fn handle(&self, _request: Request) -> Result<Response> {
        let rendered = self
            .loader
            .render(&self.template_name)
            .map_err(|e| reinhardt_exception::Error::Internal(e.to_string()))?;

        Ok(Response::ok()
            .with_body(Bytes::from(rendered))
            .with_header("content-type", "text/html; charset=utf-8"))
    }
}

// Test 1: Basic template rendering with router
#[tokio::test]
async fn test_router_with_basic_template_rendering() {
    // Create a simple template
    let template = HelloTemplate {
        name: "World".to_string(),
    };

    let handler = Arc::new(TemplateHandler::new(template));

    // Create request
    let request = Request::new(
        Method::GET,
        Uri::from_static("/hello/"),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    // Handle request
    let response = handler.handle(request).await.unwrap();

    // Verify response
    assert_eq!(response.status, StatusCode::OK);

    let body_str = String::from_utf8(response.body.to_vec()).unwrap();
    assert_eq!(body_str, "Hello World!");

    // Verify content-type header
    assert_eq!(
        response
            .headers
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap(),
        "text/html; charset=utf-8"
    );
}

// Test 2: Template rendering with context variables
#[tokio::test]
async fn test_router_template_with_context() {
    // Create template with multiple variables
    let template = UserTemplate {
        username: "alice".to_string(),
        email: "alice@example.com".to_string(),
    };

    let handler = Arc::new(TemplateHandler::new(template));

    // Create request
    let request = Request::new(
        Method::GET,
        Uri::from_static("/user/alice/"),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    // Handle request
    let response = handler.handle(request).await.unwrap();

    // Verify response
    assert_eq!(response.status, StatusCode::OK);

    let body_str = String::from_utf8(response.body.to_vec()).unwrap();
    assert_eq!(body_str, "User: alice, Email: alice@example.com");
}

// Test 3: Dynamic template loading with TemplateLoader
#[tokio::test]
async fn test_router_with_dynamic_template_loading() {
    // Create TemplateLoader
    let mut loader = TemplateLoader::new();

    // Register templates
    loader.register("home", || "Welcome to Home Page".to_string());
    loader.register("about", || "About Us Page".to_string());
    loader.register("contact", || "Contact: info@example.com".to_string());

    let loader = Arc::new(loader);

    // Test home template
    let home_handler = Arc::new(DynamicTemplateHandler::new(loader.clone(), "home"));
    let home_request = Request::new(
        Method::GET,
        Uri::from_static("/"),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    let home_response = home_handler.handle(home_request).await.unwrap();
    assert_eq!(home_response.status, StatusCode::OK);
    let home_body = String::from_utf8(home_response.body.to_vec()).unwrap();
    assert_eq!(home_body, "Welcome to Home Page");

    // Test about template
    let about_handler = Arc::new(DynamicTemplateHandler::new(loader.clone(), "about"));
    let about_request = Request::new(
        Method::GET,
        Uri::from_static("/about/"),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    let about_response = about_handler.handle(about_request).await.unwrap();
    assert_eq!(about_response.status, StatusCode::OK);
    let about_body = String::from_utf8(about_response.body.to_vec()).unwrap();
    assert_eq!(about_body, "About Us Page");

    // Test contact template
    let contact_handler = Arc::new(DynamicTemplateHandler::new(loader.clone(), "contact"));
    let contact_request = Request::new(
        Method::GET,
        Uri::from_static("/contact/"),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    let contact_response = contact_handler.handle(contact_request).await.unwrap();
    assert_eq!(contact_response.status, StatusCode::OK);
    let contact_body = String::from_utf8(contact_response.body.to_vec()).unwrap();
    assert_eq!(contact_body, "Contact: info@example.com");

    // Test template not found
    let missing_handler = Arc::new(DynamicTemplateHandler::new(loader.clone(), "missing"));
    let missing_request = Request::new(
        Method::GET,
        Uri::from_static("/missing/"),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    let missing_result = missing_handler.handle(missing_request).await;
    assert!(missing_result.is_err());
}
