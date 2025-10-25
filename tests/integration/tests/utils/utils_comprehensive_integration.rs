//! Comprehensive integration tests for reinhardt-utils
//!
//! These tests validate real-world usage scenarios that combine multiple
//! utility functions from reinhardt-utils.

use reinhardt_utils::{dateformat, encoding, html, text, timezone};

#[test]
fn test_blog_publishing_pipeline() {
    // Scenario: Publishing a blog post with sanitized content

    // 1. Generate URL slug from title
    let title = "Hello World! My First Post";
    let slug = encoding::slugify(title);
    assert_eq!(slug, "hello-world-my-first-post");

    // 2. Sanitize HTML content
    let raw_html = "<p>This is <script>alert('xss')</script> safe content</p>";
    let safe_html = html::escape(raw_html);
    assert!(!safe_html.contains("<script>"));
    assert!(safe_html.contains("&lt;script&gt;"));

    // 3. Format publication date
    let pub_date = timezone::now();
    let formatted_date = dateformat::format(&pub_date, "F j, Y");
    assert!(!formatted_date.is_empty());

    // 4. Truncate meta description
    let long_desc = "This is a very long description that needs to be truncated for SEO purposes because it's too long for meta tags";
    let meta_desc = encoding::truncate_chars(long_desc, 160);
    assert!(meta_desc.chars().count() <= 160);

    // 5. URL encode for sharing
    let share_url = format!("/blog/{}", slug);
    let encoded_url = encoding::urlencode(&share_url);
    assert_eq!(encoded_url, "%2Fblog%2Fhello-world-my-first-post");
}

#[test]
fn test_user_profile_display() {
    // Scenario: Displaying user profile with sanitized data

    // 1. Escape user input
    let user_name = "John <script>alert(1)</script> Doe";
    let safe_name = html::escape(user_name);
    assert!(!safe_name.contains("<script>"));

    // 2. Localize registration date
    let utc_date = timezone::now();
    let local_date = timezone::to_local(utc_date);
    let formatted = dateformat::format(&utc_date, "M d, Y");
    assert!(!formatted.is_empty());

    // 3. Format phone number
    let phone = "1234567890";
    let formatted_phone = text::phone_format(phone);
    assert_eq!(formatted_phone, "(123) 456-7890");

    // 4. Truncate bio
    let long_bio = "This is a very long bio that should be truncated to show only a preview on the profile page and users can click to see more";
    let bio_preview = encoding::truncate_words(long_bio, 10);
    assert!(bio_preview.ends_with("..."));
}

#[test]
fn test_multilingual_metadata() {
    // Scenario: Generating metadata for a multilingual site

    // 1. Process title
    let title = "Bonjour le Monde";
    let slug = encoding::slugify(title);
    assert_eq!(slug, "bonjour-le-monde");

    // 2. Timezone conversion
    let utc_time = timezone::now();
    let local_time = timezone::to_local(utc_time);

    // 3. Date formatting
    let date_str = dateformat::format(&utc_time, "Y-m-d H:i:s");
    assert!(date_str.contains("-"));
    assert!(date_str.contains(":"));

    // 4. URL encoding
    let url_path = format!("/{}", slug);
    let encoded = encoding::urlencode(&url_path);
    assert_eq!(encoded, "%2Fbonjour-le-monde");
}

#[test]
fn test_comment_system() {
    // Scenario: Processing and displaying user comments

    // 1. Escape comment body
    let comment = "Great post! Check this out: <a href='http://spam.com'>click here</a>";
    let safe_comment = html::escape(comment);
    assert!(!safe_comment.contains("<a"));

    // 2. Display relative time
    let now = timezone::now();
    let time_str = dateformat::format(&now, "g:i A");
    assert!(time_str.contains("M") || time_str.contains("AM") || time_str.contains("PM"));

    // 3. Sanitize username
    let username = "User<123>";
    let safe_username = html::strip_tags(username);
    assert_eq!(safe_username, "User");

    // 4. Detect and escape links
    let text_with_link = "Visit http://example.com for more info";
    let escaped = html::escape(text_with_link);
    assert!(escaped.contains("http://example.com"));
}

#[test]
fn test_api_response_generation() {
    // Scenario: Generating API response with formatted data

    // 1. Format timestamp in ISO8601
    let timestamp = timezone::now();
    let iso_string = timezone::format_datetime(&timestamp);
    assert!(iso_string.contains('T'));
    assert!(iso_string.contains('Z') || iso_string.contains('+'));

    // 2. Escape text for JSON
    let user_input = "Line 1\nLine 2\tTabbed";
    let json_safe = encoding::escapejs(user_input);
    assert!(!json_safe.contains('\n'));
    assert!(!json_safe.contains('\t'));

    // 3. Format numbers
    let count = 1234567;
    let formatted_count = text::intcomma(count);
    assert_eq!(formatted_count, "1,234,567");
}

#[test]
fn test_search_results() {
    // Scenario: Displaying search results with snippets

    // 1. Escape query
    let query = "<script>search term</script>";
    let safe_query = html::escape(query);
    assert!(!safe_query.contains("<script>"));

    // 2. Generate snippet
    let content = "This is a long piece of content that needs to be truncated to show only a relevant snippet for the search results page";
    let snippet = encoding::truncate_words(content, 15);
    let word_count = snippet.split_whitespace().filter(|w| *w != "...").count();
    assert!(word_count <= 15);

    // 3. Display timestamp
    let result_time = timezone::now();
    let time_display = dateformat::shortcuts::short_date(&result_time);
    assert!(!time_display.is_empty());
}

#[test]
fn test_form_validation() {
    // Scenario: Validating and sanitizing form input

    // 1. Sanitize text input
    let user_input = "  <b>Bold Text</b>  ";
    let sanitized = html::strip_tags(user_input);
    assert_eq!(sanitized, "  Bold Text  ");

    // 2. Format error messages
    let field_name = "email_address";
    let formatted_field = text::title(field_name);
    assert!(formatted_field.chars().next().unwrap().is_uppercase());

    // 3. Parse date input
    let date_str = "2025-01-15T10:30:00Z";
    let parsed = timezone::parse_datetime(date_str);
    assert!(parsed.is_ok());

    // 4. Validate URL
    let url = "http://example.com/path?query=value";
    let encoded = encoding::urlencode(url);
    assert!(encoded.contains("%"));
}

#[test]
fn test_data_export() {
    // Scenario: Exporting data with proper formatting

    // 1. Escape CSV content
    let csv_value = "Value with \"quotes\" and, commas";
    let escaped = html::escape(csv_value);
    assert!(escaped.contains("&quot;"));

    // 2. Convert timezone for export
    let utc_date = timezone::now();
    let export_format = dateformat::shortcuts::iso_datetime(&utc_date);
    assert!(export_format.contains("-"));
    assert!(export_format.contains(":"));

    // 3. Format numbers for display
    let amount = 1234567.89;
    let formatted = text::floatcomma(amount, 2);
    assert_eq!(formatted, "1,234,567.89");

    // 4. Normalize text
    let text = "  Multiple   Spaces  ";
    let lines = encoding::wrap_text(text.trim(), 50);
    assert!(!lines.is_empty());
}
