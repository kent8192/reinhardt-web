use reinhardt_negotiation::cache::{CacheKey, NegotiationCache};
use reinhardt_negotiation::detector::ContentTypeDetector;
use reinhardt_negotiation::encoding::{Encoding, EncodingNegotiator};
use reinhardt_negotiation::language::{Language, LanguageNegotiator};
use reinhardt_negotiation::{ContentNegotiator, MediaType};

/// Test complete request processing workflow
#[test]
fn test_complete_workflow() {
	// Setup negotiators
	let content_negotiator = ContentNegotiator::new();
	let language_negotiator = LanguageNegotiator::new();
	let encoding_negotiator = EncodingNegotiator::new();
	let mut cache: NegotiationCache<MediaType> = NegotiationCache::new();

	// Available options
	let available_media_types = vec![
		MediaType::new("application", "json"),
		MediaType::new("text", "html"),
	];
	let available_languages = vec![
		Language::new("en"),
		Language::new("fr"),
		Language::new("ja"),
	];
	let available_encodings = vec![Encoding::Gzip, Encoding::Brotli, Encoding::Identity];

	// Request headers
	let accept = "application/json";
	let accept_language = "fr;q=0.9, en;q=0.8";
	let accept_encoding = "br, gzip";

	// Content negotiation
	let cache_key = CacheKey::new(accept);
	let media_type = cache.get_or_compute(&cache_key, || {
		content_negotiator.negotiate(accept, &available_media_types)
	});

	assert_eq!(media_type.subtype, "json");

	// Language negotiation
	let language = language_negotiator.negotiate(accept_language, &available_languages);
	assert_eq!(language.code, "fr");

	// Encoding negotiation
	let encoding = encoding_negotiator.negotiate(accept_encoding, &available_encodings);
	assert_eq!(encoding, Encoding::Brotli);

	// Verify cache works
	let cached_media_type = cache.get(&cache_key);
	assert!(cached_media_type.is_some());
	assert_eq!(cached_media_type.unwrap().subtype, "json");
}

/// Test content-type detection with content negotiation
#[test]
fn test_detector_with_negotiation() {
	let detector = ContentTypeDetector::new();
	let negotiator = ContentNegotiator::new();

	// Request body is JSON
	let body = r#"{"name": "test", "value": 123}"#;
	let detected = detector.detect(body.as_bytes());

	assert_eq!(detected.subtype, "json");

	// Verify it matches available types
	let available = vec![
		MediaType::new("application", "json"),
		MediaType::new("text", "html"),
	];

	let negotiated = negotiator.negotiate(&detected.to_string(), &available);
	assert_eq!(negotiated.subtype, "json");
}

/// Test multi-language cache
#[test]
fn test_language_cache() {
	let mut cache: NegotiationCache<Language> = NegotiationCache::new();
	let negotiator = LanguageNegotiator::new();
	let available = vec![
		Language::new("en"),
		Language::new("fr"),
		Language::new("ja"),
	];

	let test_cases = vec![
		("fr, en", "fr"),
		("en", "en"),
		("ja;q=0.9, en;q=0.8", "ja"),
		("de, es, en", "en"),
	];

	for (accept_language, expected) in test_cases {
		let key = CacheKey::new(accept_language);
		let language =
			cache.get_or_compute(&key, || negotiator.negotiate(accept_language, &available));

		assert_eq!(language.code, expected);
	}

	// Verify all are cached
	assert_eq!(cache.len(), 4);
}

/// Test encoding cache
#[test]
fn test_encoding_cache() {
	let mut cache: NegotiationCache<Encoding> = NegotiationCache::new();
	let negotiator = EncodingNegotiator::new();
	let available = vec![Encoding::Gzip, Encoding::Brotli, Encoding::Identity];

	let test_cases = vec![
		("gzip, br", Encoding::Gzip), // First match wins when quality is equal
		("gzip", Encoding::Gzip),
		("identity", Encoding::Identity),
		("deflate", Encoding::Brotli), // Server preference fallback
	];

	for (accept_encoding, expected) in test_cases {
		let key = CacheKey::new(accept_encoding);
		let encoding =
			cache.get_or_compute(&key, || negotiator.negotiate(accept_encoding, &available));

		assert_eq!(encoding, expected);
	}

	assert_eq!(cache.len(), 4);
}

/// Test complex multi-header cache key
#[test]
fn test_multi_header_cache() {
	let mut cache: NegotiationCache<String> = NegotiationCache::new();

	let headers = &[
		("Accept", "application/json"),
		("Accept-Language", "en-US"),
		("Accept-Encoding", "gzip"),
	];

	let key = CacheKey::from_headers(headers);

	cache.set(key.clone(), "cached-response".to_string());

	let result = cache.get(&key);
	assert!(result.is_some());
	assert_eq!(result.unwrap(), "cached-response");

	// Different headers should produce different key
	let different_headers = &[
		("Accept", "text/html"),
		("Accept-Language", "en-US"),
		("Accept-Encoding", "gzip"),
	];

	let different_key = CacheKey::from_headers(different_headers);
	assert!(cache.get(&different_key).is_none());
}

/// Test detection with multiple content types
#[test]
fn test_detector_multiple_formats() {
	let detector = ContentTypeDetector::new();

	let test_cases = vec![
		(r#"{"key": "value"}"#, "json"),
		(r#"<root><item>value</item></root>"#, "xml"),
		("name: John\nage: 30", "yaml"),
		("name=John&age=30", "x-www-form-urlencoded"),
	];

	for (body, expected_subtype) in test_cases {
		let media_type = detector.detect(body.as_bytes());
		assert_eq!(media_type.subtype, expected_subtype);
	}
}

/// Test language negotiation with regions
#[test]
fn test_language_negotiation_with_regions() {
	let negotiator = LanguageNegotiator::new();
	let available = vec![
		Language::with_region("en", "US"),
		Language::with_region("en", "GB"),
		Language::new("fr"),
	];

	// Specific region request
	let result = negotiator.negotiate("en-US", &available);
	assert_eq!(result.tag(), "en-US");

	// General language request should match any region
	let result2 = negotiator.negotiate("en", &available);
	assert_eq!(result2.code, "en");

	// Multiple with quality
	let result3 = negotiator.negotiate("fr;q=1.0, en-US;q=0.9", &available);
	assert_eq!(result3.code, "fr");
}

/// Test encoding with quality factors
#[test]
fn test_encoding_quality_negotiation() {
	let negotiator = EncodingNegotiator::new();
	let available = vec![Encoding::Gzip, Encoding::Brotli, Encoding::Deflate];

	// Highest quality wins
	let result = negotiator.negotiate("gzip;q=0.5, br;q=1.0, deflate;q=0.3", &available);
	assert_eq!(result, Encoding::Brotli);

	// Equal quality, client order matters
	let result2 = negotiator.negotiate("deflate;q=1.0, gzip;q=1.0", &available);
	assert_eq!(result2, Encoding::Deflate);
}

/// Test full negotiation pipeline with all components
#[test]
fn test_full_pipeline() {
	// Setup all negotiators and cache
	let content_negotiator = ContentNegotiator::new();
	let language_negotiator = LanguageNegotiator::new();
	let encoding_negotiator = EncodingNegotiator::new();
	let detector = ContentTypeDetector::new();
	let mut media_cache: NegotiationCache<MediaType> = NegotiationCache::new();
	let mut lang_cache: NegotiationCache<Language> = NegotiationCache::new();
	let mut enc_cache: NegotiationCache<Encoding> = NegotiationCache::new();

	// Available resources
	let media_types = vec![
		MediaType::new("application", "json"),
		MediaType::new("text", "html"),
	];
	let languages = vec![Language::new("en"), Language::new("fr")];
	let encodings = vec![Encoding::Gzip, Encoding::Identity];

	// Request
	let request_body = r#"{"user": "john"}"#;
	let accept = "application/json";
	let accept_language = "fr";
	let accept_encoding = "gzip";

	// Step 1: Detect content type from body
	let detected_type = detector.detect(request_body.as_bytes());
	assert_eq!(detected_type.subtype, "json");

	// Step 2: Negotiate response content type (with cache)
	let media_key = CacheKey::new(accept);
	let response_media = media_cache.get_or_compute(&media_key, || {
		content_negotiator.negotiate(accept, &media_types)
	});
	assert_eq!(response_media.subtype, "json");

	// Step 3: Negotiate language (with cache)
	let lang_key = CacheKey::new(accept_language);
	let response_lang = lang_cache.get_or_compute(&lang_key, || {
		language_negotiator.negotiate(accept_language, &languages)
	});
	assert_eq!(response_lang.code, "fr");

	// Step 4: Negotiate encoding (with cache)
	let enc_key = CacheKey::new(accept_encoding);
	let response_encoding = enc_cache.get_or_compute(&enc_key, || {
		encoding_negotiator.negotiate(accept_encoding, &encodings)
	});
	assert_eq!(response_encoding, Encoding::Gzip);

	// Verify everything is cached
	assert_eq!(media_cache.len(), 1);
	assert_eq!(lang_cache.len(), 1);
	assert_eq!(enc_cache.len(), 1);
}
