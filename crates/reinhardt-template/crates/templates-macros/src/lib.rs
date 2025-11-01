//! Procedural macros for compile-time template path validation in Reinhardt.
//!
//! This crate provides macros that validate template paths at compile time,
//! ensuring that template references are safe and follow correct syntax.
//!
//! # Examples
//!
//! ```
//! use reinhardt_templates_macros::template;
//!
//! // Valid template paths
//! let path = template!("emails/welcome.html");
//! let path = template!("blog/post_detail.html");
//! let path = template!("admin/user-list.html");
//! ```
//!
//! # Compile-time Validation
//!
//! The following will fail to compile:
//!
//! ```compile_fail
//! use reinhardt_templates_macros::template;
//!
//! // Error: path contains parent directory reference
//! let path = template!("../etc/passwd");
//! ```
//!
//! ```compile_fail
//! use reinhardt_templates_macros::template;
//!
//! // Error: path contains backslash
//! let path = template!("emails\\welcome.html");
//! ```

use proc_macro::TokenStream;
use quote::quote;
use syn::{LitStr, parse_macro_input};

mod validation;

use validation::{TemplateValidationError, validate_template_path};

/// Validates template path syntax at compile time.
///
/// This macro ensures that template paths are safe and follow the correct format:
/// - Must be a relative path (no leading `/`)
/// - No parent directory references (`..`)
/// - Only forward slashes (`/`) as path separators
/// - Must have a valid file extension (`.html`, `.txt`, `.md`, etc.)
/// - No invalid characters
///
/// # Examples
///
/// ```
/// use reinhardt_templates_macros::template;
///
/// let simple = template!("base.html");
/// let nested = template!("emails/welcome.html");
/// let deep = template!("apps/blog/templates/post_detail.html");
/// ```
///
/// # Errors
///
/// This macro will produce compile-time errors for unsafe or invalid paths:
///
/// ```compile_fail
/// use reinhardt_templates_macros::template;
///
// Error: parent directory reference
/// let invalid = template!("../etc/passwd");
/// ```
///
/// ```compile_fail
/// use reinhardt_templates_macros::template;
///
// Error: backslash separator
/// let invalid = template!("emails\\welcome.html");
/// ```
///
/// ```compile_fail
/// use reinhardt_templates_macros::template;
///
// Error: absolute path
/// let invalid = template!("/etc/passwd");
/// ```
#[proc_macro]
pub fn template(input: TokenStream) -> TokenStream {
	let path_str = parse_macro_input!(input as LitStr);
	let path = path_str.value();

	// Validate template path
	if let Err(e) = validate_template_path(&path) {
		let error_msg = format_error_message(&e);
		return syn::Error::new(path_str.span(), error_msg)
			.to_compile_error()
			.into();
	}

	// Return the validated path string as-is
	quote! {
		#path_str
	}
	.into()
}

/// Formats validation errors with helpful context
fn format_error_message(error: &TemplateValidationError) -> String {
	match error {
        TemplateValidationError::ContainsParentDirectory => {
            "Template path must not contain parent directory references (..)\n\
             This is a security measure to prevent path traversal attacks.\n\
             Example: template!(\"emails/welcome.html\") instead of template!(\"../etc/passwd\")"
                .to_string()
        }
        TemplateValidationError::ContainsBackslash => {
            "Template path must use forward slashes (/) as path separators\n\
             Backslashes (\\) are not allowed for cross-platform compatibility.\n\
             Example: template!(\"emails/welcome.html\") instead of template!(\"emails\\\\welcome.html\")"
                .to_string()
        }
        TemplateValidationError::IsAbsolutePath => {
            "Template path must be relative (not absolute)\n\
             Paths should not start with '/' for portability.\n\
             Example: template!(\"emails/welcome.html\") instead of template!(\"/templates/welcome.html\")"
                .to_string()
        }
        TemplateValidationError::NoFileExtension => {
            "Template path must have a file extension\n\
             Valid extensions: .html, .htm, .txt, .md, .xml, .json, .css, .js, .svg, .rst\n\
             Example: template!(\"base.html\") instead of template!(\"base\")"
                .to_string()
        }
        TemplateValidationError::InvalidFileExtension { ext } => {
            format!(
                "Invalid file extension '.{}'\n\
                 Valid extensions: .html, .htm, .txt, .md, .xml, .json, .css, .js, .svg, .rst\n\
                 Example: template!(\"welcome.html\") instead of template!(\"script.py\")",
                ext
            )
        }
        TemplateValidationError::EmptyPath => {
            "Template path cannot be empty\n\
             Provide a valid relative path to a template file.\n\
             Example: template!(\"base.html\")"
                .to_string()
        }
        TemplateValidationError::ContainsNullByte => {
            "Template path contains null byte\n\
             Null bytes are not allowed in file paths."
                .to_string()
        }
        TemplateValidationError::InvalidCharacter { ch, position } => {
            format!(
                "Invalid character '{}' at position {}\n\
                 Template paths may only contain:\n\
                 - Alphanumeric characters (a-z, A-Z, 0-9)\n\
                 - Hyphens (-)\n\
                 - Underscores (_)\n\
                 - Slashes (/)\n\
                 - Dots (.)",
                ch, position
            )
        }
    }
}
