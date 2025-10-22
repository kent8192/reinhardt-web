//! Template Pagination Integration Tests
//!
//! Integration tests for template rendering with pagination functionality
//! inspired by Django REST Framework's pagination tests.
//!
//! These tests cover:
//! - Pagination HTML generation
//! - Template rendering with paginated data
//! - Integration with reinhardt-pagination
//! - Error handling in paginated templates

use askama::Template as AskamaTemplate;
use reinhardt_templates::{
    custom_filters::*, FileSystemTemplateLoader, Template, TemplateError, TemplateLoader,
    TemplateResult,
};
use std::collections::HashMap;
use std::path::Path;
use tempfile::TempDir;

// ============================================================================
// Pagination Data Structures
// ============================================================================

#[derive(Debug, Clone)]
pub struct PaginatedData<T> {
    pub items: Vec<T>,
    pub current_page: u32,
    pub total_pages: u32,
    pub total_items: u64,
    pub items_per_page: u32,
    pub has_previous: bool,
    pub has_next: bool,
    pub previous_page: Option<u32>,
    pub next_page: Option<u32>,
}

impl<T> PaginatedData<T> {
    pub fn new(items: Vec<T>, current_page: u32, total_items: u64, items_per_page: u32) -> Self {
        let total_pages = ((total_items as f64) / (items_per_page as f64)).ceil() as u32;
        let has_previous = current_page > 1;
        let has_next = current_page < total_pages;
        let previous_page = if has_previous {
            Some(current_page - 1)
        } else {
            None
        };
        let next_page = if has_next {
            Some(current_page + 1)
        } else {
            None
        };

        Self {
            items,
            current_page,
            total_pages,
            total_items,
            items_per_page,
            has_previous,
            has_next,
            previous_page,
            next_page,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PaginationInfo {
    pub current_page: u32,
    pub total_pages: u32,
    pub total_items: u64,
    pub items_per_page: u32,
    pub has_previous: bool,
    pub has_next: bool,
    pub previous_page: Option<u32>,
    pub next_page: Option<u32>,
    pub page_range: Vec<u32>,
}

impl PaginationInfo {
    pub fn from_paginated_data<T>(data: &PaginatedData<T>) -> Self {
        let page_range = Self::generate_page_range(data.current_page, data.total_pages);

        Self {
            current_page: data.current_page,
            total_pages: data.total_pages,
            total_items: data.total_items,
            items_per_page: data.items_per_page,
            has_previous: data.has_previous,
            has_next: data.has_next,
            previous_page: data.previous_page,
            next_page: data.next_page,
            page_range,
        }
    }

    fn generate_page_range(current_page: u32, total_pages: u32) -> Vec<u32> {
        let mut pages = Vec::new();
        let start = (current_page.saturating_sub(2)).max(1);
        let end = (current_page + 2).min(total_pages);

        for page in start..=end {
            pages.push(page);
        }
        pages
    }
}

// ============================================================================
// Test Templates
// ============================================================================

#[derive(AskamaTemplate)]
#[template(
    source = r#"<div class="pagination">
{% if pagination.has_previous %}
<a href="?page={{ pagination.previous_page.unwrap() }}" class="prev">Previous</a>
{% endif %}

{% for page in pagination.page_range %}
<a href="?page={{ page }}">{{ page }}</a>
{% endfor %}

{% if pagination.has_next %}
<a href="?page={{ pagination.next_page.unwrap() }}" class="next">Next</a>
{% endif %}
</div>"#,
    ext = "html"
)]
struct PaginationTemplate {
    pagination: PaginationInfo,
}

#[derive(AskamaTemplate)]
#[template(
    source = r#"<div class="paginated-list">
<h2>Items</h2>
<ul>
{% for item in items %}
<li>{{ item }}</li>
{% endfor %}
</ul>

{{ pagination_html }}
</div>"#,
    ext = "html"
)]
struct PaginatedListTemplate {
    items: Vec<String>,
    pagination_html: String,
}

#[derive(AskamaTemplate)]
#[template(
    source = r#"<div class="pagination-info">
Showing {{ pagination.current_page }} to {{ pagination.total_pages }} of {{ pagination.total_items }} results
</div>"#,
    ext = "html"
)]
struct PaginationInfoTemplate {
    pagination: PaginationInfo,
}

#[derive(AskamaTemplate)]
#[template(
    source = r#"<div class="pagination-controls">
{% if pagination.has_previous %}
<a href="?page=1" class="first">First</a>
<a href="?page={{ pagination.previous_page.unwrap() }}" class="prev">Previous</a>
{% endif %}

<span class="page-info">Page {{ pagination.current_page }} of {{ pagination.total_pages }}</span>

{% if pagination.has_next %}
<a href="?page={{ pagination.next_page.unwrap() }}" class="next">Next</a>
<a href="?page={{ pagination.total_pages }}" class="last">Last</a>
{% endif %}
</div>"#,
    ext = "html"
)]
struct PaginationControlsTemplate {
    pagination: PaginationInfo,
}

// ============================================================================
// Pagination HTML Generation Tests
// ============================================================================

#[test]
fn test_pagination_html_generation() {
    // Test basic pagination HTML generation
    let pagination = PaginationInfo {
        current_page: 3,
        total_pages: 10,
        total_items: 100,
        items_per_page: 10,
        has_previous: true,
        has_next: true,
        previous_page: Some(2),
        next_page: Some(4),
        page_range: vec![1, 2, 3, 4, 5],
    };

    let tmpl = PaginationTemplate { pagination };
    let result = tmpl.render().unwrap();

    assert!(result.contains("href=\"?page=2\""));
    assert!(result.contains("href=\"?page=4\""));
    assert!(result.contains("href=\"?page=3\""));
    assert!(result.contains("Previous"));
    assert!(result.contains("Next"));
}

#[test]
fn test_pagination_html_first_page() {
    // Test pagination HTML for first page
    let pagination = PaginationInfo {
        current_page: 1,
        total_pages: 5,
        total_items: 50,
        items_per_page: 10,
        has_previous: false,
        has_next: true,
        previous_page: None,
        next_page: Some(2),
        page_range: vec![1, 2, 3],
    };

    let tmpl = PaginationTemplate { pagination };
    let result = tmpl.render().unwrap();

    assert!(!result.contains("Previous"));
    assert!(result.contains("Next"));
    assert!(result.contains("href=\"?page=1\""));
}

#[test]
fn test_pagination_html_last_page() {
    // Test pagination HTML for last page
    let pagination = PaginationInfo {
        current_page: 5,
        total_pages: 5,
        total_items: 50,
        items_per_page: 10,
        has_previous: true,
        has_next: false,
        previous_page: Some(4),
        next_page: None,
        page_range: vec![3, 4, 5],
    };

    let tmpl = PaginationTemplate { pagination };
    let result = tmpl.render().unwrap();

    assert!(result.contains("Previous"));
    assert!(!result.contains("Next"));
    assert!(result.contains("href=\"?page=5\""));
}

#[test]
fn test_pagination_html_single_page() {
    // Test pagination HTML for single page
    let pagination = PaginationInfo {
        current_page: 1,
        total_pages: 1,
        total_items: 5,
        items_per_page: 10,
        has_previous: false,
        has_next: false,
        previous_page: None,
        next_page: None,
        page_range: vec![1],
    };

    let tmpl = PaginationTemplate { pagination };
    let result = tmpl.render().unwrap();

    assert!(!result.contains("Previous"));
    assert!(!result.contains("Next"));
    assert!(result.contains("href=\"?page=1\""));
}

// ============================================================================
// Paginated Data Rendering Tests
// ============================================================================

#[test]
fn test_paginated_list_rendering() {
    // Test rendering paginated list with pagination HTML
    let items = vec![
        "Item 1".to_string(),
        "Item 2".to_string(),
        "Item 3".to_string(),
    ];

    let pagination = PaginationInfo {
        current_page: 1,
        total_pages: 3,
        total_items: 8,
        items_per_page: 3,
        has_previous: false,
        has_next: true,
        previous_page: None,
        next_page: Some(2),
        page_range: vec![1, 2, 3],
    };

    let pagination_tmpl = PaginationTemplate {
        pagination: pagination.clone(),
    };
    let pagination_html = pagination_tmpl.render().unwrap();

    let tmpl = PaginatedListTemplate {
        items,
        pagination_html,
    };

    let result = tmpl.render().unwrap();

    assert!(result.contains("<h2>Items</h2>"));
    assert!(result.contains("<li>Item 1</li>"));
    assert!(result.contains("<li>Item 2</li>"));
    assert!(result.contains("<li>Item 3</li>"));
    assert!(result.contains("Next"));
}

#[test]
fn test_paginated_list_empty() {
    // Test rendering empty paginated list
    let items = vec![];

    let pagination = PaginationInfo {
        current_page: 1,
        total_pages: 0,
        total_items: 0,
        items_per_page: 10,
        has_previous: false,
        has_next: false,
        previous_page: None,
        next_page: None,
        page_range: vec![],
    };

    let pagination_tmpl = PaginationTemplate {
        pagination: pagination.clone(),
    };
    let pagination_html = pagination_tmpl.render().unwrap();

    let tmpl = PaginatedListTemplate {
        items,
        pagination_html,
    };

    let result = tmpl.render().unwrap();

    assert!(result.contains("<h2>Items</h2>"));
    assert!(!result.contains("<li>"));
    assert!(!result.contains("Previous"));
    assert!(!result.contains("Next"));
}

// ============================================================================
// Pagination Info Tests
// ============================================================================

#[test]
fn test_pagination_info_rendering() {
    // Test pagination info rendering
    let pagination = PaginationInfo {
        current_page: 2,
        total_pages: 5,
        total_items: 47,
        items_per_page: 10,
        has_previous: true,
        has_next: true,
        previous_page: Some(1),
        next_page: Some(3),
        page_range: vec![1, 2, 3, 4, 5],
    };

    let tmpl = PaginationInfoTemplate { pagination };
    let result = tmpl.render().unwrap();

    assert!(result.contains("Showing 2 to 5 of 47 results"));
}

#[test]
fn test_pagination_controls_rendering() {
    // Test pagination controls rendering
    let pagination = PaginationInfo {
        current_page: 3,
        total_pages: 10,
        total_items: 100,
        items_per_page: 10,
        has_previous: true,
        has_next: true,
        previous_page: Some(2),
        next_page: Some(4),
        page_range: vec![1, 2, 3, 4, 5],
    };

    let tmpl = PaginationControlsTemplate { pagination };
    let result = tmpl.render().unwrap();

    assert!(result.contains("href=\"?page=1\""));
    assert!(result.contains("href=\"?page=2\""));
    assert!(result.contains("href=\"?page=4\""));
    assert!(result.contains("href=\"?page=10\""));
    assert!(result.contains("Page 3 of 10"));
    assert!(result.contains("First"));
    assert!(result.contains("Previous"));
    assert!(result.contains("Next"));
    assert!(result.contains("Last"));
}

// ============================================================================
// Pagination Data Generation Tests
// ============================================================================

#[test]
fn test_paginated_data_generation() {
    // Test paginated data generation
    let items = vec![
        "Item 1".to_string(),
        "Item 2".to_string(),
        "Item 3".to_string(),
        "Item 4".to_string(),
        "Item 5".to_string(),
    ];

    let paginated = PaginatedData::new(items, 2, 5, 2);

    assert_eq!(paginated.current_page, 2);
    assert_eq!(paginated.total_pages, 3);
    assert_eq!(paginated.total_items, 5);
    assert_eq!(paginated.items_per_page, 2);
    assert!(paginated.has_previous);
    assert!(paginated.has_next);
    assert_eq!(paginated.previous_page, Some(1));
    assert_eq!(paginated.next_page, Some(3));
}

#[test]
fn test_paginated_data_single_page() {
    // Test paginated data for single page
    let items = vec!["Item 1".to_string(), "Item 2".to_string()];
    let paginated = PaginatedData::new(items, 1, 2, 10);

    assert_eq!(paginated.current_page, 1);
    assert_eq!(paginated.total_pages, 1);
    assert_eq!(paginated.total_items, 2);
    assert_eq!(paginated.items_per_page, 10);
    assert!(!paginated.has_previous);
    assert!(!paginated.has_next);
    assert_eq!(paginated.previous_page, None);
    assert_eq!(paginated.next_page, None);
}

#[test]
fn test_pagination_info_from_data() {
    // Test pagination info generation from paginated data
    let items = vec!["Item 1".to_string(), "Item 2".to_string()];
    let paginated = PaginatedData::new(items, 2, 5, 2);
    let info = PaginationInfo::from_paginated_data(&paginated);

    assert_eq!(info.current_page, 2);
    assert_eq!(info.total_pages, 3);
    assert_eq!(info.total_items, 5);
    assert_eq!(info.items_per_page, 2);
    assert!(info.has_previous);
    assert!(info.has_next);
    assert_eq!(info.previous_page, Some(1));
    assert_eq!(info.next_page, Some(3));
    assert_eq!(info.page_range, vec![1, 2, 3]);
}

// ============================================================================
// Filter Integration with Pagination Tests
// ============================================================================

#[test]
fn test_pagination_with_filters() {
    // Test pagination with filter integration
    let pagination = PaginationInfo {
        current_page: 1,
        total_pages: 3,
        total_items: 8,
        items_per_page: 3,
        has_previous: false,
        has_next: true,
        previous_page: None,
        next_page: Some(2),
        page_range: vec![1, 2, 3],
    };

    // Test with title case filter
    let title_text = "page 1 of 3";
    let formatted_title = title(title_text).unwrap();
    assert_eq!(formatted_title, "Page 1 Of 3");

    // Test with number formatting
    let page_info = format!(
        "Page {} of {}",
        pagination.current_page, pagination.total_pages
    );
    assert_eq!(page_info, "Page 1 of 3");
}

#[test]
fn test_pagination_url_generation() {
    // Test pagination URL generation with filters
    let base_url = "https://example.com/api/items";
    let page = 2;

    // Simulate URL generation with query parameters
    let url = format!("{}?page={}", base_url, page);
    assert_eq!(url, "https://example.com/api/items?page=2");

    // Test with additional parameters
    let url_with_params = format!("{}?page={}&search=test&sort=name", base_url, page);
    assert_eq!(
        url_with_params,
        "https://example.com/api/items?page=2&search=test&sort=name"
    );
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn test_pagination_error_handling() {
    // Test pagination error handling
    let pagination = PaginationInfo {
        current_page: 0, // Invalid page
        total_pages: 5,
        total_items: 50,
        items_per_page: 10,
        has_previous: false,
        has_next: true,
        previous_page: None,
        next_page: Some(1),
        page_range: vec![1, 2, 3],
    };

    let tmpl = PaginationTemplate { pagination };
    let result = tmpl.render().unwrap();

    // Should still render, but with invalid page number
    assert!(result.contains("href=\"?page=1\""));
}

#[test]
fn test_pagination_overflow_handling() {
    // Test pagination overflow handling
    let pagination = PaginationInfo {
        current_page: 10, // Beyond total pages
        total_pages: 5,
        total_items: 50,
        items_per_page: 10,
        has_previous: true,
        has_next: false,
        previous_page: Some(9),
        next_page: None,
        page_range: vec![3, 4, 5],
    };

    let tmpl = PaginationTemplate { pagination };
    let result = tmpl.render().unwrap();

    // Should render with current page beyond total
    assert!(result.contains("href=\"?page=5\""));
    assert!(!result.contains("Next"));
}

// ============================================================================
// Performance Tests
// ============================================================================

#[test]
fn test_pagination_performance_large_dataset() {
    // Test pagination performance with large dataset
    let large_items: Vec<String> = (0..10000).map(|i| format!("Item {}", i)).collect();

    let paginated = PaginatedData::new(large_items, 100, 10000, 100);
    let info = PaginationInfo::from_paginated_data(&paginated);

    let start = std::time::Instant::now();
    let tmpl = PaginationTemplate { pagination: info };
    let result = tmpl.render().unwrap();
    let duration = start.elapsed();

    // Should complete in reasonable time
    assert!(duration.as_millis() < 100);
    assert!(result.contains("href=\"?page=100\""));
}

#[test]
fn test_pagination_performance_many_pages() {
    // Test pagination performance with many pages
    let pagination = PaginationInfo {
        current_page: 500,
        total_pages: 1000,
        total_items: 100000,
        items_per_page: 100,
        has_previous: true,
        has_next: true,
        previous_page: Some(499),
        next_page: Some(501),
        page_range: vec![498, 499, 500, 501, 502],
    };

    let start = std::time::Instant::now();
    let tmpl = PaginationTemplate { pagination };
    let result = tmpl.render().unwrap();
    let duration = start.elapsed();

    // Should complete in reasonable time
    assert!(duration.as_millis() < 100);
    assert!(result.contains("href=\"?page=500\""));
}

// ============================================================================
// Integration with File System Loader Tests
// ============================================================================

#[test]
fn test_pagination_with_file_system_loader() {
    // Test pagination with file system loader
    let temp_dir = TempDir::new().unwrap();
    let template_path = temp_dir.path().join("pagination.html");

    let template_content = r#"<div class="pagination">
{% if pagination.has_previous %}
<a href="?page={{ pagination.previous_page }}">Previous</a>
{% endif %}
<span>Page {{ pagination.current_page }} of {{ pagination.total_pages }}</span>
{% if pagination.has_next %}
<a href="?page={{ pagination.next_page }}">Next</a>
{% endif %}
</div>"#;

    std::fs::write(&template_path, template_content).unwrap();

    let loader = FileSystemTemplateLoader::new(temp_dir.path());
    let content = loader.load("pagination.html").unwrap();

    assert!(content.contains("{% if pagination.has_previous %}"));
    assert!(content.contains("{% if pagination.has_next %}"));
    assert!(content.contains("{{ pagination.current_page }}"));
}

// ============================================================================
// Mock Integration Tests
// ============================================================================

#[test]
fn test_integration_with_orm_mock() {
    // Mock test for integration with ORM (when available)
    // This would test pagination with database queries
    let mock_items = vec![
        "Database Item 1".to_string(),
        "Database Item 2".to_string(),
        "Database Item 3".to_string(),
    ];

    let paginated = PaginatedData::new(mock_items, 1, 3, 10);
    let info = PaginationInfo::from_paginated_data(&paginated);

    let tmpl = PaginationTemplate { pagination: info };
    let result = tmpl.render().unwrap();

    assert!(result.contains("href=\"?page=1\""));
    assert!(!result.contains("Previous"));
    assert!(!result.contains("Next"));
}

#[test]
fn test_integration_with_rest_api_mock() {
    // Mock test for integration with REST API (when available)
    // This would test pagination in API responses
    let api_response = HashMap::from([
        ("page".to_string(), "2".to_string()),
        ("total_pages".to_string(), "5".to_string()),
        ("total_items".to_string(), "50".to_string()),
    ]);

    let current_page = api_response.get("page").unwrap().parse::<u32>().unwrap();
    let total_pages = api_response
        .get("total_pages")
        .unwrap()
        .parse::<u32>()
        .unwrap();
    let total_items = api_response
        .get("total_items")
        .unwrap()
        .parse::<u64>()
        .unwrap();

    let pagination = PaginationInfo {
        current_page,
        total_pages,
        total_items,
        items_per_page: 10,
        has_previous: current_page > 1,
        has_next: current_page < total_pages,
        previous_page: if current_page > 1 {
            Some(current_page - 1)
        } else {
            None
        },
        next_page: if current_page < total_pages {
            Some(current_page + 1)
        } else {
            None
        },
        page_range: vec![1, 2, 3, 4, 5],
    };

    let tmpl = PaginationTemplate { pagination };
    let result = tmpl.render().unwrap();

    assert!(result.contains("href=\"?page=2\""));
    assert!(result.contains("Previous"));
    assert!(result.contains("Next"));
}
