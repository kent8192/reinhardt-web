//! Template loader tests
//!
//! Tests for template loading functionality inspired by Django's test_loaders.py

use crate::{Template as AskamaTemplate, TemplateError, TemplateId, TemplateLoader};
use askama::Template;

#[derive(Template)]
#[template(source = "Test template {{ value }}", ext = "txt")]
struct TestTemplate {
    value: String,
}

#[derive(Template)]
#[template(source = "Another template {{ data }}", ext = "txt")]
struct AnotherTemplate {
    data: String,
}

struct HomeTemplateId;
impl TemplateId for HomeTemplateId {
    const NAME: &'static str = "home.html";
}

struct AboutTemplateId;
impl TemplateId for AboutTemplateId {
    const NAME: &'static str = "about.html";
}

#[test]
fn test_template_loader_get_template() {
    // Test basic template loading similar to Django's test_get_template
    let mut loader = TemplateLoader::new();
    loader.register("test.html", || {
        let tmpl = TestTemplate {
            value: "Hello".to_string(),
        };
        tmpl.render().unwrap()
    });

    let result = loader.render("test.html");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Test template Hello");
}

#[test]
fn test_template_loader_missing() {
    // Test missing template handling similar to Django's test_file_does_not_exist
    let loader = TemplateLoader::new();

    let result = loader.render("nonexistent.html");
    assert!(result.is_err(), "Expected error for nonexistent template");

    match result {
        Err(TemplateError::TemplateNotFound(name)) => {
            assert_eq!(name, "nonexistent.html");
        }
        Err(e) => {
            panic!("Expected TemplateNotFound error, got: {:?}", e);
        }
        Ok(_) => {
            panic!("Expected error but got Ok");
        }
    }
}

#[test]
fn test_template_loader_multiple_templates() {
    // Test loading multiple templates
    let mut loader = TemplateLoader::new();

    loader.register("template1.html", || {
        let tmpl = TestTemplate {
            value: "First".to_string(),
        };
        tmpl.render().unwrap()
    });

    loader.register("template2.html", || {
        let tmpl = AnotherTemplate {
            data: "Second".to_string(),
        };
        tmpl.render().unwrap()
    });

    assert_eq!(
        loader.render("template1.html").unwrap(),
        "Test template First"
    );
    assert_eq!(
        loader.render("template2.html").unwrap(),
        "Another template Second"
    );
}

#[test]
fn test_typed_template_loading() {
    // Test type-safe template loading
    let mut loader = TemplateLoader::new();

    loader.register_typed::<HomeTemplateId, _>(|| {
        let tmpl = TestTemplate {
            value: "Home Page".to_string(),
        };
        tmpl.render().unwrap()
    });

    let result = loader.render_typed::<HomeTemplateId>();
    assert!(result.is_ok());
    assert!(result.unwrap().contains("Home Page"));
}

#[test]
fn test_typed_template_not_found() {
    // Test type-safe template not found error
    let loader = TemplateLoader::new();

    let result = loader.render_typed::<AboutTemplateId>();
    assert!(result.is_err());

    if let Err(TemplateError::TemplateNotFound(name)) = result {
        assert_eq!(name, "about.html");
    }
}

#[test]
fn test_template_loader_override() {
    // Test overriding a template (similar to Django's caching tests)
    let mut loader = TemplateLoader::new();

    loader.register("override.html", || "First version".to_string());

    assert_eq!(loader.render("override.html").unwrap(), "First version");

    // Override with new version
    loader.register("override.html", || "Second version".to_string());

    assert_eq!(loader.render("override.html").unwrap(), "Second version");
}

#[test]
fn test_template_name_with_special_characters() {
    // Test template names with special characters
    let mut loader = TemplateLoader::new();

    loader.register("template-with-dash.html", || "Dash template".to_string());
    loader.register("template_with_underscore.html", || {
        "Underscore template".to_string()
    });
    loader.register("template.with.dots.html", || "Dots template".to_string());

    assert_eq!(
        loader.render("template-with-dash.html").unwrap(),
        "Dash template"
    );
    assert_eq!(
        loader.render("template_with_underscore.html").unwrap(),
        "Underscore template"
    );
    assert_eq!(
        loader.render("template.with.dots.html").unwrap(),
        "Dots template"
    );
}

#[test]
fn test_template_loader_empty_name() {
    // Test loading template with empty name
    let loader = TemplateLoader::new();

    let result = loader.render("");
    assert!(result.is_err());

    if let Err(TemplateError::TemplateNotFound(name)) = result {
        assert_eq!(name, "");
    }
}

#[test]
fn test_template_loader_mixed_registration() {
    // Test mixing typed and untyped registration
    let mut loader = TemplateLoader::new();

    loader.register_typed::<HomeTemplateId, _>(|| "Home".to_string());
    loader.register("manual.html", || "Manual".to_string());

    assert_eq!(loader.render_typed::<HomeTemplateId>().unwrap(), "Home");
    assert_eq!(loader.render("manual.html").unwrap(), "Manual");
    assert_eq!(loader.render("home.html").unwrap(), "Home");
}
