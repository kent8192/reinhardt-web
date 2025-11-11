use reinhardt_negotiation::language::{Language, LanguageNegotiator};

#[test]
fn test_language_parse_simple() {
	let lang = Language::parse("en").unwrap();
	assert_eq!(lang.code, "en");
	assert_eq!(lang.region, None);
	assert_eq!(lang.quality, 1.0);
}

#[test]
fn test_language_parse_with_region() {
	let lang = Language::parse("en-US").unwrap();
	assert_eq!(lang.code, "en");
	assert_eq!(lang.region, Some("US".to_string()));
	assert_eq!(lang.quality, 1.0);
}

#[test]
fn test_language_parse_with_quality() {
	let lang = Language::parse("fr;q=0.9").unwrap();
	assert_eq!(lang.code, "fr");
	assert_eq!(lang.quality, 0.9);
}

#[test]
fn test_language_parse_with_region_and_quality() {
	let lang = Language::parse("ja-JP;q=0.8").unwrap();
	assert_eq!(lang.code, "ja");
	assert_eq!(lang.region, Some("JP".to_string()));
	assert_eq!(lang.quality, 0.8);
}

#[test]
fn test_language_tag() {
	let en = Language::new("en");
	assert_eq!(en.tag(), "en");

	let en_us = Language::with_region("en", "US");
	assert_eq!(en_us.tag(), "en-US");
}

#[test]
fn test_language_matches() {
	let en = Language::new("en");
	let en_us = Language::with_region("en", "US");
	let en_gb = Language::with_region("en", "GB");
	let fr = Language::new("fr");

	// Same language without region matches
	assert!(en.matches(&en_us));
	assert!(en_us.matches(&en));

	// Different regions but same language matches
	assert!(en_us.matches(&en));
	assert!(en_gb.matches(&en));

	// Exact region match
	assert!(en_us.matches(&en_us));

	// Different language doesn't match
	assert!(!en.matches(&fr));
	assert!(!en_us.matches(&fr));
}

#[test]
fn test_negotiator_simple() {
	let negotiator = LanguageNegotiator::new();
	let available = vec![
		Language::new("en"),
		Language::new("fr"),
		Language::new("ja"),
	];

	let result = negotiator.negotiate("fr", &available);
	assert_eq!(result.code, "fr");
}

#[test]
fn test_negotiator_with_quality() {
	let negotiator = LanguageNegotiator::new();
	let available = vec![
		Language::new("en"),
		Language::new("fr"),
		Language::new("ja"),
	];

	// Higher quality should win
	let result = negotiator.negotiate("fr;q=0.5, en;q=0.9", &available);
	assert_eq!(result.code, "en");
}

#[test]
fn test_negotiator_with_region() {
	let negotiator = LanguageNegotiator::new();
	let available = vec![
		Language::with_region("en", "US"),
		Language::with_region("en", "GB"),
		Language::new("fr"),
	];

	// Request for en-US should match en-US
	let result = negotiator.negotiate("en-US", &available);
	assert_eq!(result.code, "en");
	assert_eq!(result.region, Some("US".to_string()));

	// Request for en should match any en variant
	let result2 = negotiator.negotiate("en", &available);
	assert_eq!(result2.code, "en");
}

#[test]
fn test_negotiator_fallback() {
	let negotiator = LanguageNegotiator::new();
	let available = vec![Language::new("en"), Language::new("fr")];

	// No match should return fallback (en)
	let result = negotiator.negotiate("de, es", &available);
	assert_eq!(result.code, "en");
}

#[test]
fn test_negotiator_custom_fallback() {
	let negotiator = LanguageNegotiator::with_fallback(Language::new("ja"));
	let available = vec![Language::new("ja"), Language::new("fr")];

	// No match should return custom fallback (ja)
	let result = negotiator.negotiate("de", &available);
	assert_eq!(result.code, "ja");
}

#[test]
fn test_negotiator_complex() {
	let negotiator = LanguageNegotiator::new();
	let available = vec![
		Language::new("en"),
		Language::new("fr"),
		Language::new("ja"),
	];

	// Complex Accept-Language header
	let result = negotiator.negotiate("ja;q=0.5, fr;q=0.8, en;q=1.0", &available);
	assert_eq!(result.code, "en"); // Highest quality
}

#[test]
fn test_find_all_matches() {
	let negotiator = LanguageNegotiator::new();
	let available = vec![
		Language::new("en"),
		Language::new("fr"),
		Language::new("ja"),
	];

	let matches = negotiator.find_all_matches("en, fr, de", &available);
	assert_eq!(matches.len(), 2);
	assert_eq!(matches[0].code, "en");
	assert_eq!(matches[1].code, "fr");
}

#[test]
fn test_parse_accept_language() {
	let negotiator = LanguageNegotiator::new();
	let languages = negotiator.parse_accept_language("en-US, fr;q=0.9, ja;q=0.8");

	assert_eq!(languages.len(), 3);
	assert_eq!(languages[0].code, "en");
	assert_eq!(languages[0].region, Some("US".to_string()));
	assert_eq!(languages[1].code, "fr");
	assert_eq!(languages[1].quality, 0.9);
	assert_eq!(languages[2].code, "ja");
	assert_eq!(languages[2].quality, 0.8);
}
