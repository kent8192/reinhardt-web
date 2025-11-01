//! Content negotiation tests based on Django REST Framework
//!
//! These tests verify that content negotiation works correctly for:
//! - Accept header parsing and matching
//! - Media type parameter handling
//! - Renderer selection
//! - Error handling

use reinhardt_negotiation::{
	BaseContentNegotiation, BaseNegotiator, ContentNegotiator, MediaType, NegotiationError,
	RendererInfo,
};

// Mock renderers for testing
fn create_mock_renderers() -> Vec<MediaType> {
	vec![
		{
			let mt = MediaType::new("application", "json");
			mt
		},
		{
			let mt = MediaType::new("text", "html");
			mt
		},
		{
			let mut mt = MediaType::new("application", "openapi+json");
			mt.parameters
				.push(("version".to_string(), "2.0".to_string()));
			mt
		},
	]
}

// Test: Client without Accept header uses first renderer
#[test]
fn test_client_without_accept_use_renderer() {
	let negotiator = ContentNegotiator::new();
	let renderers = create_mock_renderers();

	let result = negotiator.select_renderer(None, &renderers);
	assert!(result.is_ok());

	let (_, accepted_media_type) = result.unwrap();
	assert_eq!(accepted_media_type, "application/json");
}

// Test: Client with underspecified Accept header (*/*) uses first renderer
#[test]
fn test_client_underspecifies_accept_use_renderer() {
	let negotiator = ContentNegotiator::new();
	let renderers = create_mock_renderers();

	let result = negotiator.select_renderer(Some("*/*"), &renderers);
	assert!(result.is_ok());

	let (_, accepted_media_type) = result.unwrap();
	assert_eq!(accepted_media_type, "application/json");
}

// Test: Client overspecifies Accept header with parameters
#[test]
fn test_client_overspecifies_accept_use_client() {
	let negotiator = ContentNegotiator::new();
	let renderers = create_mock_renderers();

	let result = negotiator.select_renderer(Some("application/json; indent=8"), &renderers);
	assert!(result.is_ok());

	let (_, accepted_media_type) = result.unwrap();
	assert_eq!(accepted_media_type, "application/json; indent=8");
}

// Test: Client specifies parameters in Accept header
#[test]
fn test_client_specifies_parameter() {
	let negotiator = ContentNegotiator::new();
	let renderers = create_mock_renderers();

	let result =
		negotiator.select_renderer(Some("application/openapi+json;version=2.0"), &renderers);
	assert!(result.is_ok());

	let (matched_renderer, accepted_media_type) = result.unwrap();
	assert_eq!(accepted_media_type, "application/openapi+json; version=2.0");
	assert_eq!(matched_renderer.subtype, "openapi+json");
}

// Test: MediaType match returns false if main types don't match
#[test]
fn test_match_is_false_if_main_types_not_match() {
	let mediatype = MediaType::new("test_1", "subtype");
	let another_mediatype = MediaType::new("test_2", "subtype");
	assert!(!mediatype.matches(&another_mediatype));
}

// Test: MediaType match returns false if parameter keys don't match
#[test]
fn test_mediatype_match_is_false_if_keys_not_match() {
	let mut mediatype = MediaType::new("application", "json");
	mediatype
		.parameters
		.push(("test_param".to_string(), "foo".to_string()));

	let mut another_mediatype = MediaType::new("application", "json");
	another_mediatype
		.parameters
		.push(("test_param".to_string(), "bar".to_string()));

	assert!(!mediatype.matches(&another_mediatype));
}

// Test: MediaType precedence with wildcard subtype
#[test]
fn test_mediatype_precedence_with_wildcard_subtype() {
	let mediatype = MediaType::new("test", "*");
	assert_eq!(mediatype.precedence(), 2); // Has specific main type but wildcard subtype
}

// Test: MediaType string representation
#[test]
fn test_mediatype_string_representation() {
	let mut mediatype = MediaType::new("test", "*");
	mediatype
		.parameters
		.push(("foo".to_string(), "bar".to_string()));

	let result = mediatype.full_string();
	assert_eq!(result, "test/*; foo=bar");
}

// Test: Raise error if no suitable renderers found
#[test]
fn test_raise_error_if_no_suitable_renderers_found() {
	let negotiator = ContentNegotiator::new();
	let renderers = vec![RendererInfo {
		media_type: MediaType::new("application", "xml"),
		format: "xml".to_string(),
	}];

	let result = negotiator.filter_renderers(&renderers, "json");
	assert!(result.is_err());
	assert_eq!(result.unwrap_err(), NegotiationError::NoSuitableRenderer);
}

// Test: Base content negotiation raises error for abstract select_parser method
#[test]
fn test_raise_error_for_abstract_select_parser_method() {
	let negotiator = BaseNegotiator;
	let result = negotiator.select_parser(None, &[]);
	assert!(result.is_err());
	assert_eq!(result.unwrap_err(), NegotiationError::NoSuitableRenderer);
}

// Test: Base content negotiation raises error for abstract select_renderer method
#[test]
fn test_raise_error_for_abstract_select_renderer_method() {
	let negotiator = BaseNegotiator;
	let result = negotiator.select_renderer(None, &[]);
	assert!(result.is_err());
	assert_eq!(result.unwrap_err(), NegotiationError::NoSuitableRenderer);
}
