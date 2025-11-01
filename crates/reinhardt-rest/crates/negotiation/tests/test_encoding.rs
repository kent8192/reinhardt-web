use reinhardt_negotiation::encoding::{Encoding, EncodingNegotiator, EncodingQuality};

#[test]
fn test_encoding_parse() {
	assert_eq!(Encoding::parse("gzip"), Some(Encoding::Gzip));
	assert_eq!(Encoding::parse("x-gzip"), Some(Encoding::Gzip));
	assert_eq!(Encoding::parse("br"), Some(Encoding::Brotli));
	assert_eq!(Encoding::parse("deflate"), Some(Encoding::Deflate));
	assert_eq!(Encoding::parse("identity"), Some(Encoding::Identity));
	assert_eq!(Encoding::parse("*"), Some(Encoding::Identity));
	assert_eq!(Encoding::parse("unknown"), None);
}

#[test]
fn test_encoding_as_str() {
	assert_eq!(Encoding::Gzip.as_str(), "gzip");
	assert_eq!(Encoding::Brotli.as_str(), "br");
	assert_eq!(Encoding::Deflate.as_str(), "deflate");
	assert_eq!(Encoding::Identity.as_str(), "identity");
}

#[test]
fn test_encoding_display() {
	assert_eq!(format!("{}", Encoding::Gzip), "gzip");
	assert_eq!(format!("{}", Encoding::Brotli), "br");
}

#[test]
fn test_encoding_quality_parse_simple() {
	let enc = EncodingQuality::parse("gzip").unwrap();
	assert_eq!(enc.encoding, Encoding::Gzip);
	assert_eq!(enc.quality, 1.0);
}

#[test]
fn test_encoding_quality_parse_with_quality() {
	let enc = EncodingQuality::parse("gzip;q=0.9").unwrap();
	assert_eq!(enc.encoding, Encoding::Gzip);
	assert_eq!(enc.quality, 0.9);
}

#[test]
fn test_encoding_quality_new() {
	let enc = EncodingQuality::new(Encoding::Gzip);
	assert_eq!(enc.encoding, Encoding::Gzip);
	assert_eq!(enc.quality, 1.0);
}

#[test]
fn test_encoding_quality_with_quality() {
	let enc = EncodingQuality::with_quality(Encoding::Brotli, 0.8);
	assert_eq!(enc.encoding, Encoding::Brotli);
	assert_eq!(enc.quality, 0.8);
}

#[test]
fn test_negotiator_simple() {
	let negotiator = EncodingNegotiator::new();
	let available = vec![Encoding::Gzip, Encoding::Identity];

	let result = negotiator.negotiate("gzip", &available);
	assert_eq!(result, Encoding::Gzip);
}

#[test]
fn test_negotiator_multiple_encodings() {
	let negotiator = EncodingNegotiator::new();
	let available = vec![Encoding::Gzip, Encoding::Brotli, Encoding::Identity];

	// Client accepts multiple with equal quality, first match wins
	let result = negotiator.negotiate("gzip, br, deflate", &available);
	assert_eq!(result, Encoding::Gzip); // First match in client's list
}

#[test]
fn test_negotiator_with_quality() {
	let negotiator = EncodingNegotiator::new();
	let available = vec![Encoding::Gzip, Encoding::Identity];

	// Client prefers identity over gzip
	let result = negotiator.negotiate("gzip;q=0.5, identity;q=1.0", &available);
	assert_eq!(result, Encoding::Identity);
}

#[test]
fn test_negotiator_fallback() {
	let negotiator = EncodingNegotiator::new();
	let available = vec![Encoding::Identity];

	// Client requests unavailable encodings, should fallback to identity
	let result = negotiator.negotiate("br, gzip, deflate", &available);
	assert_eq!(result, Encoding::Identity);
}

#[test]
fn test_negotiator_custom_preference() {
	let negotiator = EncodingNegotiator::with_preference(vec![
		Encoding::Gzip,
		Encoding::Deflate,
		Encoding::Identity,
	]);
	let available = vec![Encoding::Gzip, Encoding::Brotli, Encoding::Deflate];

	// Server prefers Gzip over Brotli
	let result = negotiator.negotiate("br, gzip, deflate", &available);
	assert_eq!(result, Encoding::Brotli); // Client accepts all, so client header order matters
}

#[test]
fn test_parse_accept_encoding() {
	let negotiator = EncodingNegotiator::new();
	let encodings = negotiator.parse_accept_encoding("gzip, deflate;q=0.8, br;q=0.9");

	assert_eq!(encodings.len(), 3);
	assert_eq!(encodings[0].encoding, Encoding::Gzip);
	assert_eq!(encodings[0].quality, 1.0);
	assert_eq!(encodings[1].encoding, Encoding::Deflate);
	assert_eq!(encodings[1].quality, 0.8);
	assert_eq!(encodings[2].encoding, Encoding::Brotli);
	assert_eq!(encodings[2].quality, 0.9);
}

#[test]
fn test_select_best() {
	let negotiator = EncodingNegotiator::new();
	let available = vec![Encoding::Gzip, Encoding::Brotli, Encoding::Identity];

	// Brotli has highest quality
	let result = negotiator.select_best("br;q=1.0, gzip;q=0.9", &available);
	assert_eq!(result, Encoding::Brotli);

	// Gzip has highest quality
	let result2 = negotiator.select_best("gzip;q=1.0, br;q=0.9", &available);
	assert_eq!(result2, Encoding::Gzip);
}

#[test]
fn test_negotiator_identity_fallback() {
	let negotiator = EncodingNegotiator::new();
	let available = vec![Encoding::Gzip, Encoding::Brotli];

	// Client requests deflate which is not available, fallback to server preference
	let result = negotiator.negotiate("deflate", &available);
	assert_eq!(result, Encoding::Brotli); // Server prefers Brotli over Gzip
}

#[test]
fn test_negotiator_complex_scenario() {
	let negotiator = EncodingNegotiator::new();
	let available = vec![Encoding::Gzip, Encoding::Brotli, Encoding::Deflate];

	// Complex header with multiple qualities
	let result = negotiator.negotiate(
		"br;q=0.9, gzip;q=0.8, deflate;q=0.7, identity;q=0.1",
		&available,
	);
	assert_eq!(result, Encoding::Brotli); // Highest quality available
}
