//! HTTP i18n tests
//!
//! Tests based on Django's i18n/tests.py HTTP-related tests

use bytes::Bytes;
use hyper::{
	HeaderMap, Method, Version,
	header::{ACCEPT_LANGUAGE, COOKIE},
};
use reinhardt_apps::Request;

fn create_test_request_with_accept_language(accept_lang: &str) -> Request {
	let mut headers = HeaderMap::new();
	headers.insert(ACCEPT_LANGUAGE, accept_lang.parse().unwrap());

	Request::new(
		Method::GET,
		"/".parse().unwrap(),
		Version::HTTP_11,
		headers,
		Bytes::new(),
	)
}

fn create_test_request_with_cookie(cookie: &str) -> Request {
	let mut headers = HeaderMap::new();
	headers.insert(COOKIE, cookie.parse().unwrap());

	Request::new(
		Method::GET,
		"/".parse().unwrap(),
		Version::HTTP_11,
		headers,
		Bytes::new(),
	)
}

#[test]
fn test_parse_accept_language_simple() {
	let req = create_test_request_with_accept_language("en-US");
	let languages = req.get_accepted_languages();

	assert_eq!(languages.len(), 1);
	assert_eq!(languages[0].0, "en-US");
	assert_eq!(languages[0].1, 1.0);
}

#[test]
fn test_parse_accept_language_multiple() {
	let req = create_test_request_with_accept_language("en-US,en;q=0.9,ja;q=0.8");
	let languages = req.get_accepted_languages();

	assert_eq!(languages.len(), 3);
	assert_eq!(languages[0].0, "en-US");
	assert_eq!(languages[0].1, 1.0);
	assert_eq!(languages[1].0, "en");
	assert_eq!(languages[1].1, 0.9);
	assert_eq!(languages[2].0, "ja");
	assert_eq!(languages[2].1, 0.8);
}

#[test]
fn test_parse_accept_language_with_spaces() {
	let req = create_test_request_with_accept_language("en-US, en;q=0.9, ja;q=0.8");
	let languages = req.get_accepted_languages();

	assert_eq!(languages.len(), 3);
	assert_eq!(languages[0].0, "en-US");
	assert_eq!(languages[1].0, "en");
	assert_eq!(languages[2].0, "ja");
}

#[test]
fn test_parse_accept_language_quality_ordering() {
	let req = create_test_request_with_accept_language("ja;q=0.8,en;q=0.9,en-US");
	let languages = req.get_accepted_languages();

	// Should be sorted by quality descending
	assert_eq!(languages[0].0, "en-US"); // q=1.0 (default)
	assert_eq!(languages[1].0, "en"); // q=0.9
	assert_eq!(languages[2].0, "ja"); // q=0.8
}

#[test]
fn test_get_preferred_language() {
	let req = create_test_request_with_accept_language("ja;q=0.8,en;q=0.9,fr;q=1.0");
	let preferred = req.get_preferred_language();

	assert_eq!(preferred, Some("fr".to_string()));
}

#[test]
fn test_get_preferred_language_no_header() {
	let req = Request::new(
		Method::GET,
		"/".parse().unwrap(),
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);

	let preferred = req.get_preferred_language();
	assert_eq!(preferred, None);
}

#[test]
fn test_valid_language_codes() {
	// Valid codes
	let req1 = create_test_request_with_accept_language("en");
	assert_eq!(req1.get_accepted_languages().len(), 1);

	let req2 = create_test_request_with_accept_language("en-US");
	assert_eq!(req2.get_accepted_languages().len(), 1);

	let req3 = create_test_request_with_accept_language("zh-Hans");
	assert_eq!(req3.get_accepted_languages().len(), 1);

	let req4 = create_test_request_with_accept_language("sr-Latn-RS");
	assert_eq!(req4.get_accepted_languages().len(), 1);

	let req5 = create_test_request_with_accept_language("nl-nl-x-informal");
	assert_eq!(req5.get_accepted_languages().len(), 1);
}

#[test]
fn test_invalid_language_codes() {
	// Starting with hyphen
	let req1 = create_test_request_with_accept_language("-en");
	assert_eq!(req1.get_accepted_languages().len(), 0);

	// Ending with hyphen
	let req2 = create_test_request_with_accept_language("en-");
	assert_eq!(req2.get_accepted_languages().len(), 0);

	// Invalid characters
	let req3 = create_test_request_with_accept_language("en_US");
	assert_eq!(req3.get_accepted_languages().len(), 0);

	// Empty
	let req4 = create_test_request_with_accept_language("");
	assert_eq!(req4.get_accepted_languages().len(), 0);
}

#[test]
fn test_language_code_too_long() {
	// 256 characters - should be rejected
	let long_code = "a".repeat(256);
	let req = create_test_request_with_accept_language(&long_code);
	assert_eq!(req.get_accepted_languages().len(), 0);

	// 255 characters - should be accepted
	let valid_code = "a".repeat(255);
	let req2 = create_test_request_with_accept_language(&valid_code);
	assert_eq!(req2.get_accepted_languages().len(), 1);
}

#[test]
fn test_get_language_from_cookie() {
	let req = create_test_request_with_cookie("reinhardt_language=ja; sessionid=abc123");
	let lang = req.get_language_from_cookie("reinhardt_language");

	assert_eq!(lang, Some("ja".to_string()));
}

#[test]
fn test_get_language_from_cookie_not_found() {
	let req = create_test_request_with_cookie("sessionid=abc123");
	let lang = req.get_language_from_cookie("reinhardt_language");

	assert_eq!(lang, None);
}

#[test]
fn test_get_language_from_cookie_multiple() {
	let req = create_test_request_with_cookie("theme=dark; reinhardt_language=fr; sessionid=xyz");
	let lang = req.get_language_from_cookie("reinhardt_language");

	assert_eq!(lang, Some("fr".to_string()));
}

#[test]
fn test_get_language_from_cookie_invalid_language() {
	// Cookie with invalid language code should be rejected
	let req = create_test_request_with_cookie("reinhardt_language=en_US");
	let lang = req.get_language_from_cookie("reinhardt_language");

	assert_eq!(lang, None);
}

#[test]
fn test_parse_accept_language_with_wildcard() {
	let req = create_test_request_with_accept_language("en-US,en;q=0.9,*;q=0.5");
	let languages = req.get_accepted_languages();

	// Wildcard should be treated as invalid and filtered out
	assert_eq!(languages.len(), 2);
	assert_eq!(languages[0].0, "en-US");
	assert_eq!(languages[1].0, "en");
}

#[test]
fn test_parse_accept_language_complex() {
	// Complex real-world example
	let req =
		create_test_request_with_accept_language("fr-CH, fr;q=0.9, en;q=0.8, de;q=0.7, *;q=0.5");
	let languages = req.get_accepted_languages();

	assert_eq!(languages[0].0, "fr-CH");
	assert_eq!(languages[0].1, 1.0);
	assert_eq!(languages[1].0, "fr");
	assert_eq!(languages[1].1, 0.9);
	assert_eq!(languages[2].0, "en");
	assert_eq!(languages[2].1, 0.8);
	assert_eq!(languages[3].0, "de");
	assert_eq!(languages[3].1, 0.7);
}

#[test]
fn test_parse_accept_language_malformed_quality() {
	let req = create_test_request_with_accept_language("en;q=abc,fr;q=0.8");
	let languages = req.get_accepted_languages();

	// Malformed quality should default to 1.0
	assert_eq!(languages.len(), 2);
	assert_eq!(languages[0].0, "en");
	assert_eq!(languages[0].1, 1.0); // Defaulted
	assert_eq!(languages[1].0, "fr");
	assert_eq!(languages[1].1, 0.8);
}

#[test]
fn test_no_accept_language_header() {
	let req = Request::new(
		Method::GET,
		"/".parse().unwrap(),
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);

	let languages = req.get_accepted_languages();
	assert_eq!(languages.len(), 0);
}
