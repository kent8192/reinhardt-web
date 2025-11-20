# utils-core

Core utilities for Reinhardt framework

## Overview

`utils-core` provides fundamental utility functions and types used across the Reinhardt framework, including HTML manipulation, type conversions, and core abstractions inspired by Django's utility modules.

## Features

### Implemented ✓

#### HTML Utilities

- **HTML escaping** - Escape/unescape HTML special characters for XSS prevention
- **Tag stripping** - Remove HTML tags from text
- **Space normalization** - Strip spaces between HTML tags
- **Attribute escaping** - Escape values for safe use in HTML attributes
- **Template formatting** - Simple placeholder-based HTML templating
- **Conditional escaping** - Context-aware HTML escaping
- **SafeString** - Mark strings as safe to bypass autoescaping
- **HTML truncation** - Truncate HTML content to specified word count while preserving tags

#### Common Utilities

- Type conversion helpers
- String manipulation utilities
- Collection helpers
- Time and date utilities
- Encoding/decoding utilities
- Core abstraction types

## Installation

```toml
[dependencies]
utils-core = "0.1.0-alpha.1"
```

## Usage Examples

### HTML Escaping

Escape HTML special characters to prevent XSS attacks:

```rust
use reinhardt_utils_core::html::escape;

// Basic escaping
assert_eq!(escape("Hello, World!"), "Hello, World!");

// Escape script tags
assert_eq!(
    escape("<script>alert('XSS')</script>"),
    "&lt;script&gt;alert(&#x27;XSS&#x27;)&lt;/script&gt;"
);

// Escape comparison operators
assert_eq!(escape("5 < 10 & 10 > 5"), "5 &lt; 10 &amp; 10 &gt; 5");

// Escape quotes
assert_eq!(escape("\"quoted\""), "&quot;quoted&quot;");
```

**Escaped Characters:**
- `&` → `&amp;`
- `<` → `&lt;`
- `>` → `&gt;`
- `"` → `&quot;`
- `'` → `&#x27;`

### HTML Unescaping

Convert HTML entities back to their original characters:

```rust
use reinhardt_utils_core::html::unescape;

assert_eq!(unescape("&lt;div&gt;"), "<div>");
assert_eq!(unescape("&amp;"), "&");
assert_eq!(unescape("&quot;test&quot;"), "\"test\"");
assert_eq!(unescape("&#x27;"), "'");
assert_eq!(unescape("&#39;"), "'");  // Decimal entity

// Numeric entities also supported
assert_eq!(unescape("&#60;"), "<");  // Decimal
```

### Stripping HTML Tags

Remove all HTML tags from text, keeping only content:

```rust
use reinhardt_utils_core::html::strip_tags;

assert_eq!(strip_tags("<p>Hello <b>World</b></p>"), "Hello World");
assert_eq!(strip_tags("<a href=\"#\">Link</a>"), "Link");
assert_eq!(strip_tags("No tags here"), "No tags here");

// Nested tags
assert_eq!(strip_tags("<div><span>Test</span></div>"), "Test");
```

### Stripping Spaces Between Tags

Remove whitespace between HTML tags for minification:

```rust
use reinhardt_utils_core::html::strip_spaces_between_tags;

assert_eq!(
    strip_spaces_between_tags("<div>  <span>Test</span>  </div>"),
    "<div><span>Test</span></div>"
);

assert_eq!(
    strip_spaces_between_tags("<p>\n\n<b>Bold</b>\n\n</p>"),
    "<p><b>Bold</b></p>"
);
```

### Attribute Escaping

Escape values for safe use in HTML attributes:

```rust
use reinhardt_utils_core::html::escape_attr;

assert_eq!(escape_attr("value"), "value");

// Escape quotes
assert_eq!(
    escape_attr("value with \"quotes\""),
    "value with &quot;quotes&quot;"
);

// Escape newlines and tabs
assert_eq!(escape_attr("line\nbreak"), "line&#10;break");
assert_eq!(escape_attr("tab\there"), "tab&#9;here");
assert_eq!(escape_attr("test\rvalue"), "test&#13;value");
```

**Escaped in Attributes:**
- All characters from `escape()` above
- `\n` → `&#10;`
- `\r` → `&#13;`
- `\t` → `&#9;`

### HTML Template Formatting

Simple placeholder-based templating:

```rust
use reinhardt_utils_core::html::format_html;

let template = "<div class=\"{class}\">{content}</div>";
let args = [("class", "container"), ("content", "Hello")];

assert_eq!(
    format_html(template, &args),
    "<div class=\"container\">Hello</div>"
);

// Multiple placeholders
let template2 = "<div id=\"{id}\" class=\"{class}\">{content}</div>";
let args2 = [("id", "main"), ("class", "container"), ("content", "Hello")];

assert_eq!(
    format_html(template2, &args2),
    "<div id=\"main\" class=\"container\">Hello</div>"
);
```

### Conditional Escaping

Context-aware HTML escaping based on autoescape flag:

```rust
use reinhardt_utils_core::html::conditional_escape;

// Escape when autoescape is true
assert_eq!(conditional_escape("<script>", true), "&lt;script&gt;");

// Don't escape when autoescape is false
assert_eq!(conditional_escape("<script>", false), "<script>");

// Regular text unaffected
assert_eq!(conditional_escape("Hello", true), "Hello");
```

**Returns:** `Cow<'_, str>` - Borrows when no escaping needed, owns when escaped

### SafeString - Bypass Autoescaping

Mark strings as safe to prevent automatic escaping:

```rust
use reinhardt_utils_core::html::SafeString;

let safe = SafeString::new("<b>Bold</b>");
assert_eq!(safe.as_str(), "<b>Bold</b>");

// From String
let s = String::from("<i>Italic</i>");
let safe2 = SafeString::from(s);
assert_eq!(safe2.as_str(), "<i>Italic</i>");

// From &str
let safe3 = SafeString::from("<u>Underline</u>");
assert_eq!(safe3.as_str(), "<u>Underline</u>");
```

**Use Cases:**
- Template rendering with pre-escaped HTML
- Rendering trusted HTML content
- Bypassing template autoescaping for specific values

### Truncating HTML

Truncate HTML content to specified word count while preserving tags:

```rust
use reinhardt_utils_core::html::truncate_html_words;

let html = "<p>This is a <b>test</b> sentence with many words.</p>";
let truncated = truncate_html_words(html, 5);

// Keeps first 5 words + "..."
assert!(truncated.contains("This"));
assert!(truncated.contains("is"));
assert!(truncated.contains("..."));

// Preserves HTML structure
let html2 = "<div>Hello <strong>world</strong> test</div>";
let truncated2 = truncate_html_words(html2, 2);

assert!(truncated2.contains("<div>"));
assert!(truncated2.contains("<strong>"));
```

**Features:**
- Preserves HTML tag structure
- Counts only text content words
- Adds "..." suffix when truncated
- Handles nested tags correctly

## API Reference

### Escaping Functions

#### `escape(text: &str) -> String`

Escapes HTML special characters (`&`, `<`, `>`, `"`, `'`).

**Use Case:** Preventing XSS attacks in user-generated content.

#### `unescape(text: &str) -> String`

Converts HTML entities to their original characters.

**Supported Entities:**
- Named: `&amp;`, `&lt;`, `&gt;`, `&quot;`, `&#x27;`, `&apos;`
- Numeric: `&#39;` (decimal), `&#x27;` (hexadecimal)

#### `escape_attr(text: &str) -> String`

Escapes values for safe use in HTML attributes, including newlines and tabs.

**Use Case:** Safely inserting user input into HTML attributes.

### Tag Manipulation

#### `strip_tags(html: &str) -> String`

Removes all HTML tags, keeping only text content.

**Use Case:** Extracting plain text from HTML, generating text previews.

#### `strip_spaces_between_tags(html: &str) -> String`

Removes whitespace between HTML tags for minification.

**Use Case:** Reducing HTML size, cleaning up generated HTML.

### Template Functions

#### `format_html(template: &str, args: &[(&str, &str)]) -> String`

Simple placeholder replacement in HTML templates.

**Placeholder Format:** `{key}` replaced with corresponding value from args.

#### `conditional_escape(text: &str, autoescape: bool) -> Cow<'_, str>`

Conditionally escapes based on autoescape flag.

**Returns:**
- `Cow::Borrowed(text)` when `autoescape = false`
- `Cow::Owned(escape(text))` when `autoescape = true`

### SafeString Type

```rust
pub struct SafeString(String);

impl SafeString {
    pub fn new(s: impl Into<String>) -> Self;
    pub fn as_str(&self) -> &str;
}
```

**Trait Implementations:**
- `From<String>`
- `From<&str>`
- `Debug`, `Clone`

### Content Truncation

#### `truncate_html_words(html: &str, num_words: usize) -> String`

Truncates HTML to specified word count, preserving tag structure.

**Algorithm:**
1. Counts only text content words (skips tags)
2. Preserves opening/closing tags
3. Adds "..." suffix when truncated
4. Maintains HTML validity

## Testing

The crate includes comprehensive tests:

**Unit Tests:**
- Basic functionality tests for all functions
- Edge cases (empty strings, multibyte characters, nested tags)
- Error conditions (incomplete entities, unknown entities)

**Property-Based Tests (proptest):**
- `prop_escape_no_special_chars` - Validates escaping removes special characters
- `prop_strip_tags_no_angle_brackets` - Ensures tags are completely removed
- `prop_truncate_html_words_respects_limit` - Verifies word count limits
- `prop_conditional_escape_when_true` - Tests autoescape behavior
- `prop_safe_string_roundtrip` - Validates SafeString preserves content

Run tests with:
```bash
cargo test -p utils-core
cargo test -p utils-core -- --nocapture  # With output
```

## Integration with Reinhardt

HTML utilities are used throughout the framework:

**Template Rendering:**
```rust
use reinhardt_utils_core::html::{escape, SafeString};

// Escape user input in templates
let user_input = "<script>alert('XSS')</script>";
let safe_output = escape(user_input);

// Mark trusted HTML as safe
let trusted_html = "<b>Important</b>";
let safe = SafeString::new(trusted_html);
```

**View Rendering:**
```rust
use reinhardt_utils_core::html::strip_tags;

// Generate plain text preview
let html_content = "<p>Article content with <b>formatting</b></p>";
let preview = strip_tags(html_content);
```

**Form Handling:**
```rust
use reinhardt_utils_core::html::escape_attr;

// Safely insert user data into form attributes
let user_value = "value with \"quotes\"";
let safe_attr = escape_attr(user_value);
// Use in: <input value="{safe_attr}">
```

## Performance Considerations

**Escaping Functions:**
- Pre-allocate output buffer with capacity
- Single-pass character iteration
- No unnecessary allocations for strings without special characters (via `Cow`)

**Tag Stripping:**
- O(n) complexity with single pass
- In-place state tracking (no regex)
- Efficient for large HTML documents

**Truncation:**
- Preserves HTML structure with minimal overhead
- Word counting excludes tags
- Stops processing after reaching word limit

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
