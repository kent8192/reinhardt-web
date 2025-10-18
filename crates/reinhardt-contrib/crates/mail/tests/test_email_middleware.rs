//! Middleware integration tests
//!
//! Tests for email integration with reinhardt-middleware for request context.

use reinhardt_mail::{EmailBackend, EmailMessage, MemoryBackend};

/// Mock request for testing
#[derive(Clone)]
struct MockRequest {
    host: String,
    scheme: String,
    path: String,
}

impl MockRequest {
    fn new(host: &str, scheme: &str, path: &str) -> Self {
        Self {
            host: host.to_string(),
            scheme: scheme.to_string(),
            path: path.to_string(),
        }
    }

    fn get_host(&self) -> &str {
        &self.host
    }

    fn build_absolute_uri(&self, path: &str) -> String {
        format!("{}://{}{}", self.scheme, self.host, path)
    }
}

/// Email context from request
struct EmailContext {
    site_domain: Option<String>,
    scheme: Option<String>,
}

impl EmailContext {
    fn from_request(request: &MockRequest) -> Self {
        Self {
            site_domain: Some(request.get_host().to_string()),
            scheme: Some(request.scheme.clone()),
        }
    }

    fn build_absolute_url(&self, path: &str) -> String {
        let scheme = self.scheme.as_deref().unwrap_or("https");
        let domain = self.site_domain.as_deref().unwrap_or("example.com");
        format!("{}://{}{}", scheme, domain, path)
    }
}

/// Email message builder with request context
struct EmailWithContext {
    message: EmailMessage,
    context: EmailContext,
}

impl EmailWithContext {
    fn new(message: EmailMessage, context: EmailContext) -> Self {
        Self { message, context }
    }

    fn with_request_context(message: EmailMessage, request: &MockRequest) -> Self {
        let context = EmailContext::from_request(request);
        Self { message, context }
    }

    fn add_link(&mut self, link_text: &str, path: &str) {
        let absolute_url = self.context.build_absolute_url(path);
        let new_body = format!("{}\n\n{}: {}", self.message.body, link_text, absolute_url);
        self.message.body = new_body;
    }

    fn build(self) -> EmailMessage {
        self.message
    }
}

#[tokio::test]
async fn test_email_with_request_context() {
    let request = MockRequest::new("example.com", "https", "/");

    let backend = MemoryBackend::new();
    let message = EmailMessage::new()
        .subject("Password Reset")
        .body("Click the link below to reset your password:")
        .from("noreply@example.com")
        .to(vec!["user@example.com"])
        .build()
        .unwrap();

    // Add request context
    let mut email_with_context = EmailWithContext::with_request_context(message, &request);

    // Add absolute URL from request
    email_with_context.add_link("Reset Password", "/reset-password?token=abc123");

    let final_message = email_with_context.build();

    backend.send(&final_message).await.unwrap();

    let messages = backend.get_messages();
    assert_eq!(messages.len(), 1);

    // Verify absolute URL was added
    assert!(messages[0]
        .body
        .contains("https://example.com/reset-password?token=abc123"));
}

#[tokio::test]
async fn test_extract_site_domain_from_request() {
    let request1 = MockRequest::new("example.com", "https", "/");
    let request2 = MockRequest::new("subdomain.example.com", "http", "/api");
    let request3 = MockRequest::new("localhost:8000", "http", "/");

    let context1 = EmailContext::from_request(&request1);
    let context2 = EmailContext::from_request(&request2);
    let context3 = EmailContext::from_request(&request3);

    assert_eq!(context1.site_domain.as_deref(), Some("example.com"));
    assert_eq!(
        context2.site_domain.as_deref(),
        Some("subdomain.example.com")
    );
    assert_eq!(context3.site_domain.as_deref(), Some("localhost:8000"));

    // Test URL building
    assert_eq!(
        context1.build_absolute_url("/verify"),
        "https://example.com/verify"
    );
    assert_eq!(
        context2.build_absolute_url("/api/verify"),
        "http://subdomain.example.com/api/verify"
    );
    assert_eq!(
        context3.build_absolute_url("/test"),
        "http://localhost:8000/test"
    );
}

#[tokio::test]
async fn test_email_with_multiple_links() {
    let request = MockRequest::new("myapp.com", "https", "/dashboard");

    let backend = MemoryBackend::new();
    let message = EmailMessage::new()
        .subject("Account Verification")
        .body("Welcome to our service!")
        .from("welcome@myapp.com")
        .to(vec!["newuser@example.com"])
        .build()
        .unwrap();

    let mut email_with_context = EmailWithContext::with_request_context(message, &request);

    // Add multiple links
    email_with_context.add_link("Verify Email", "/verify?code=xyz789");
    email_with_context.add_link("Go to Dashboard", "/dashboard");
    email_with_context.add_link("Settings", "/settings/profile");

    let final_message = email_with_context.build();

    backend.send(&final_message).await.unwrap();

    let messages = backend.get_messages();
    assert_eq!(messages.len(), 1);

    // Verify all links are present with correct domain
    let body = &messages[0].body;
    assert!(body.contains("https://myapp.com/verify?code=xyz789"));
    assert!(body.contains("https://myapp.com/dashboard"));
    assert!(body.contains("https://myapp.com/settings/profile"));
}

#[tokio::test]
async fn test_context_with_different_schemes() {
    let http_request = MockRequest::new("test.com", "http", "/");
    let https_request = MockRequest::new("secure.com", "https", "/");

    let http_context = EmailContext::from_request(&http_request);
    let https_context = EmailContext::from_request(&https_request);

    assert_eq!(
        http_context.build_absolute_url("/link"),
        "http://test.com/link"
    );
    assert_eq!(
        https_context.build_absolute_url("/link"),
        "https://secure.com/link"
    );
}
