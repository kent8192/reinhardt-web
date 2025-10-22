//! Template Logging Integration Tests
//!
//! Integration tests for reinhardt-logging's template error logging
//! working with reinhardt-templates. These tests verify that template
//! rendering errors and variable resolution issues are properly logged.
//!
//! Tests for logging template rendering errors and variable resolution issues.
//! Based on Django's VariableResolveLoggingTests.

use reinhardt_logging::handlers::MemoryHandler;
use reinhardt_logging::{LogLevel, Logger};
use reinhardt_templates::{TemplateError, TemplateLoader, TemplateResult};
use std::sync::Arc;

/// Template context for dynamic variable resolution
pub struct TemplateContext {
    variables: std::collections::HashMap<String, serde_json::Value>,
    logger: Option<Arc<Logger>>,
    template_name: String,
}

impl TemplateContext {
    pub fn new(template_name: impl Into<String>) -> Self {
        Self {
            variables: std::collections::HashMap::new(),
            logger: None,
            template_name: template_name.into(),
        }
    }

    pub fn with_logger(mut self, logger: Arc<Logger>) -> Self {
        self.logger = Some(logger);
        self
    }

    pub fn set(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.variables.insert(key.into(), value);
    }

    /// Resolve a variable path like "article.section"
    pub async fn resolve(&self, path: &str) -> TemplateResult<serde_json::Value> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = self.variables.get(parts[0]);

        if current.is_none() {
            if let Some(logger) = &self.logger {
                logger
                    .debug(format!(
                        "Exception while resolving variable '{}' in template '{}'.",
                        parts[0], self.template_name
                    ))
                    .await;
            }
            return Err(TemplateError::TemplateNotFound(format!(
                "Variable '{}' not found",
                parts[0]
            )));
        }

        let mut value = current.unwrap().clone();

        // Traverse the path
        for (i, part) in parts.iter().enumerate().skip(1) {
            if let Some(obj) = value.as_object() {
                if let Some(next_value) = obj.get(*part) {
                    value = next_value.clone();
                } else {
                    // Variable lookup failed
                    if let Some(logger) = &self.logger {
                        logger
                            .error(format!(
                                "Exception while resolving variable '{}' in template '{}'.",
                                part, self.template_name
                            ))
                            .await;
                    }
                    return Err(TemplateError::TemplateNotFound(format!(
                        "Failed lookup for key [{}] in object at path: {}",
                        part,
                        parts[..=i].join(".")
                    )));
                }
            } else {
                if let Some(logger) = &self.logger {
                    logger
                        .error(format!(
                            "Exception while resolving variable '{}' in template '{}'.",
                            part, self.template_name
                        ))
                        .await;
                }
                return Err(TemplateError::TemplateNotFound(format!(
                    "Cannot access property '{}' on non-object",
                    part
                )));
            }
        }

        // Successful resolution - no logging
        Ok(value)
    }
}

#[tokio::test]
async fn test_log_on_variable_does_not_exist_silent() {
    // Silent failures (exceptions marked as silent) should log at DEBUG level
    let logger = Arc::new(Logger::new("reinhardt.template".to_string()));
    let handler = MemoryHandler::new(LogLevel::Debug);
    let memory = handler.clone();

    logger.add_handler(Box::new(handler)).await;
    logger.set_level(LogLevel::Debug).await;

    let mut context = TemplateContext::new("template_name");
    context = context.with_logger(logger.clone());

    // Try to access a non-existent variable (silent failure)
    context.set(
        "article".to_string(),
        serde_json::json!({"section": "News"}),
    );

    // Access non-existent property - this would be a silent failure in Django
    // (when the exception has silent_variable_failure = True)
    // We simulate this by looking up a missing key
    let result = context.resolve("missing_var").await;
    assert!(result.is_err());

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let records = memory.get_records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].level, LogLevel::Debug);
    assert!(records[0]
        .message
        .contains("Exception while resolving variable 'missing_var'"));
    assert!(records[0].message.contains("template 'template_name'"));
}

#[tokio::test]
async fn test_log_on_variable_does_not_exist_not_silent() {
    // Non-silent failures should log at ERROR level and raise exception
    let logger = Arc::new(Logger::new("reinhardt.template".to_string()));
    let handler = MemoryHandler::new(LogLevel::Debug);
    let memory = handler.clone();

    logger.add_handler(Box::new(handler)).await;
    logger.set_level(LogLevel::Debug).await;

    let mut context = TemplateContext::new("unknown");
    context = context.with_logger(logger.clone());

    context.set(
        "article".to_string(),
        serde_json::json!({"section": "News"}),
    );

    // Try to access article.author when only article.section exists
    let result = context.resolve("article.author").await;
    assert!(result.is_err());

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let records = memory.get_records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].level, LogLevel::Error);
    assert!(records[0]
        .message
        .contains("Exception while resolving variable 'author'"));
    assert!(records[0].message.contains("template 'unknown'"));

    // Verify error message contains details
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Failed lookup for key [author]"));
}

#[tokio::test]
async fn test_no_log_when_variable_exists() {
    // Successful variable resolution should not log anything
    let logger = Arc::new(Logger::new("reinhardt.template".to_string()));
    let handler = MemoryHandler::new(LogLevel::Debug);
    let memory = handler.clone();

    logger.add_handler(Box::new(handler)).await;
    logger.set_level(LogLevel::Debug).await;

    let mut context = TemplateContext::new("template_name");
    context = context.with_logger(logger.clone());

    context.set(
        "article".to_string(),
        serde_json::json!({"section": "News"}),
    );

    // Access existing property - should succeed without logging
    let result = context.resolve("article.section").await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "News");

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let records = memory.get_records();
    // No logging should occur for successful resolution
    assert_eq!(records.len(), 0);
}

#[tokio::test]
async fn test_template_loader_with_logging() {
    // Test that TemplateLoader errors can be logged
    let logger = Arc::new(Logger::new("reinhardt.template".to_string()));
    let handler = MemoryHandler::new(LogLevel::Debug);
    let memory = handler.clone();

    logger.add_handler(Box::new(handler)).await;
    logger.set_level(LogLevel::Debug).await;

    let loader = TemplateLoader::new();

    // Try to render a non-existent template
    let result = loader.render("non_existent.html");
    if result.is_err() {
        logger
            .error(format!(
                "Template not found: {}",
                result.unwrap_err().to_string()
            ))
            .await;
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let records = memory.get_records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].level, LogLevel::Error);
    assert!(records[0].message.contains("Template not found"));
}

#[tokio::test]
async fn test_nested_variable_resolution() {
    // Test deeply nested variable resolution with logging
    let logger = Arc::new(Logger::new("reinhardt.template".to_string()));
    let handler = MemoryHandler::new(LogLevel::Debug);
    let memory = handler.clone();

    logger.add_handler(Box::new(handler)).await;
    logger.set_level(LogLevel::Debug).await;

    let mut context = TemplateContext::new("nested_template");
    context = context.with_logger(logger.clone());

    context.set(
        "user".to_string(),
        serde_json::json!({
            "profile": {
                "address": {
                    "city": "Tokyo"
                }
            }
        }),
    );

    // Successful nested access
    let result = context.resolve("user.profile.address.city").await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Tokyo");

    // Failed nested access
    let result = context.resolve("user.profile.address.country").await;
    assert!(result.is_err());

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let records = memory.get_records();
    // Only the failed access should log
    assert_eq!(records.len(), 1);
    assert!(records[0].message.contains("'country'"));
}

#[tokio::test]
async fn test_context_without_logger() {
    // Context should work even without a logger attached
    let mut context = TemplateContext::new("no_logger_template");

    context.set("name".to_string(), serde_json::json!("Test"));

    // Successful access
    let result = context.resolve("name").await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Test");

    // Failed access - should still return error, just without logging
    let result = context.resolve("missing").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_multiple_failed_resolutions() {
    // Multiple failures should each log separately
    let logger = Arc::new(Logger::new("reinhardt.template".to_string()));
    let handler = MemoryHandler::new(LogLevel::Debug);
    let memory = handler.clone();

    logger.add_handler(Box::new(handler)).await;
    logger.set_level(LogLevel::Debug).await;

    let mut context = TemplateContext::new("multi_fail_template");
    context = context.with_logger(logger.clone());

    context.set("data".to_string(), serde_json::json!({"field1": "value1"}));

    // Try multiple failed resolutions
    let _ = context.resolve("data.field2").await;
    let _ = context.resolve("data.field3").await;
    let _ = context.resolve("missing_root").await;

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let records = memory.get_records();
    assert_eq!(records.len(), 3);

    // Check that each failure was logged
    assert!(records[0].message.contains("'field2'"));
    assert!(records[1].message.contains("'field3'"));
    assert!(records[2].message.contains("'missing_root'"));
}
