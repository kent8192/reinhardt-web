//! Integration tests for ViewSet pagination support
//!
//! Tests automatic pagination integration in ViewSets.

use reinhardt_viewsets::{ModelViewSet, PaginatedViewSet, PaginationConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestItem {
	id: i64,
	name: String,
}

#[tokio::test]
async fn test_default_pagination_config() {
	let viewset: ModelViewSet<TestItem, ()> = ModelViewSet::new("items");
	let config = viewset.get_pagination_config();

	assert!(config.is_some(), "Default pagination should be enabled");
}

#[tokio::test]
async fn test_custom_page_number_pagination() {
	let viewset: ModelViewSet<TestItem, ()> =
		ModelViewSet::new("items").with_pagination(PaginationConfig::page_number(20, Some(100)));

	let config = viewset.get_pagination_config();
	assert!(config.is_some());

	match config.unwrap() {
		PaginationConfig::PageNumber {
			page_size,
			max_page_size,
		} => {
			assert_eq!(page_size, 20);
			assert_eq!(max_page_size, Some(100));
		}
		_ => panic!("Expected PageNumber pagination config"),
	}
}

#[tokio::test]
async fn test_limit_offset_pagination() {
	let viewset: ModelViewSet<TestItem, ()> =
		ModelViewSet::new("items").with_pagination(PaginationConfig::limit_offset(25, Some(500)));

	let config = viewset.get_pagination_config();
	assert!(config.is_some());

	match config.unwrap() {
		PaginationConfig::LimitOffset {
			default_limit,
			max_limit,
		} => {
			assert_eq!(default_limit, 25);
			assert_eq!(max_limit, Some(500));
		}
		_ => panic!("Expected LimitOffset pagination config"),
	}
}

#[tokio::test]
async fn test_cursor_pagination() {
	let viewset: ModelViewSet<TestItem, ()> =
		ModelViewSet::new("items").with_pagination(PaginationConfig::cursor(50, "created_at"));

	let config = viewset.get_pagination_config();
	assert!(config.is_some());

	match config.unwrap() {
		PaginationConfig::Cursor {
			page_size,
			ordering_field,
		} => {
			assert_eq!(page_size, 50);
			assert_eq!(ordering_field, "created_at");
		}
		_ => panic!("Expected Cursor pagination config"),
	}
}

#[tokio::test]
async fn test_disabled_pagination() {
	let viewset: ModelViewSet<TestItem, ()> = ModelViewSet::new("items").without_pagination();

	let config = viewset.get_pagination_config();
	assert!(
		config.is_none(),
		"Pagination should be disabled after calling without_pagination()"
	);
}

#[tokio::test]
async fn test_pagination_none_config() {
	let viewset: ModelViewSet<TestItem, ()> =
		ModelViewSet::new("items").with_pagination(PaginationConfig::none());

	let config = viewset.get_pagination_config();
	assert!(config.is_some());

	match config.unwrap() {
		PaginationConfig::None => {
			// Expected - no pagination
		}
		_ => panic!("Expected None pagination config"),
	}
}

#[tokio::test]
async fn test_readonly_viewset_pagination() {
	use reinhardt_viewsets::ReadOnlyModelViewSet;

	let viewset: ReadOnlyModelViewSet<TestItem, ()> = ReadOnlyModelViewSet::new("items")
		.with_pagination(PaginationConfig::page_number(15, Some(50)));

	let config = viewset.get_pagination_config();
	assert!(config.is_some());

	match config.unwrap() {
		PaginationConfig::PageNumber {
			page_size,
			max_page_size,
		} => {
			assert_eq!(page_size, 15);
			assert_eq!(max_page_size, Some(50));
		}
		_ => panic!("Expected PageNumber pagination config"),
	}
}

#[tokio::test]
async fn test_builder_pattern_chaining() {
	let viewset: ModelViewSet<TestItem, ()> = ModelViewSet::new("items")
		.with_lookup_field("slug")
		.with_pagination(PaginationConfig::limit_offset(30, None))
		.without_pagination()
		.with_pagination(PaginationConfig::page_number(10, Some(100)));

	// Last pagination config should win
	let config = viewset.get_pagination_config();
	assert!(config.is_some());

	match config.unwrap() {
		PaginationConfig::PageNumber {
			page_size,
			max_page_size,
		} => {
			assert_eq!(page_size, 10);
			assert_eq!(max_page_size, Some(100));
		}
		_ => panic!("Expected PageNumber pagination config"),
	}
}
