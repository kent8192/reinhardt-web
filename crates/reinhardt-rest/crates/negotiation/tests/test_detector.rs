use reinhardt_negotiation::MediaType;
use reinhardt_negotiation::detector::ContentTypeDetector;

#[test]
fn test_detect_json_object() {
	let detector = ContentTypeDetector::new();
	let json = r#"{"name": "John", "age": 30}"#;
	let media_type = detector.detect(json.as_bytes());

	assert_eq!(media_type.type_, "application");
	assert_eq!(media_type.subtype, "json");
}

#[test]
fn test_detect_json_array() {
	let detector = ContentTypeDetector::new();
	let json = r#"[1, 2, 3, 4]"#;
	let media_type = detector.detect(json.as_bytes());

	assert_eq!(media_type.subtype, "json");
}

#[test]
fn test_detect_xml_with_declaration() {
	let detector = ContentTypeDetector::new();
	let xml = r#"<?xml version="1.0" encoding="UTF-8"?><root><item>value</item></root>"#;
	let media_type = detector.detect(xml.as_bytes());

	assert_eq!(media_type.type_, "application");
	assert_eq!(media_type.subtype, "xml");
}

#[test]
fn test_detect_xml_without_declaration() {
	let detector = ContentTypeDetector::new();
	let xml = r#"<root><item>value</item></root>"#;
	let media_type = detector.detect(xml.as_bytes());

	assert_eq!(media_type.subtype, "xml");
}

#[test]
fn test_detect_yaml() {
	let detector = ContentTypeDetector::new();
	let yaml = r#"
name: John Doe
age: 30
address:
  city: Tokyo
  country: Japan
"#;
	let media_type = detector.detect(yaml.as_bytes());

	assert_eq!(media_type.type_, "application");
	assert_eq!(media_type.subtype, "yaml");
}

#[test]
fn test_detect_form_data() {
	let detector = ContentTypeDetector::new();
	let form = "name=John+Doe&age=30&city=Tokyo";
	let media_type = detector.detect(form.as_bytes());

	assert_eq!(media_type.type_, "application");
	assert_eq!(media_type.subtype, "x-www-form-urlencoded");
}

#[test]
fn test_detect_empty_body() {
	let detector = ContentTypeDetector::new();
	let media_type = detector.detect(b"");

	assert_eq!(media_type.subtype, "octet-stream");
}

#[test]
fn test_custom_default() {
	let detector = ContentTypeDetector::with_default(MediaType::new("text", "plain"));
	let media_type = detector.detect(b"");

	assert_eq!(media_type.type_, "text");
	assert_eq!(media_type.subtype, "plain");
}

#[test]
fn test_detect_mixed_content() {
	let detector = ContentTypeDetector::new();

	// Test that JSON is preferred over YAML when content is ambiguous
	let json_like = r#"{"key": "value"}"#;
	let media_type = detector.detect(json_like.as_bytes());
	assert_eq!(media_type.subtype, "json");
}
