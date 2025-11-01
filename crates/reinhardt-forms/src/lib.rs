//! Form processing and validation for Reinhardt
//!
//! This crate provides comprehensive form processing capabilities including:
//! - CSRF protection with cryptographic token generation and rotation
//! - SameSite cookie support for enhanced security
//! - Origin and referer validation
//! - Chunked upload support with progress tracking
//! - File handling with MIME type detection and checksum verification
//! - XSS protection and honeypot fields
//! - Form wizards and formsets

pub mod bound_field;
pub mod chunked_upload;
pub mod csrf;
pub mod field;
pub mod fields;
pub mod file_handling;
pub mod form;
pub mod formset;
pub mod formsets;
pub mod media;
pub mod model_form;
pub mod model_formset;
pub mod security;
pub mod wizard;
pub mod xss_protection;

pub use bound_field::BoundField;
pub use chunked_upload::{
	ChunkedUploadError, ChunkedUploadManager, ChunkedUploadSession, UploadProgress,
};
pub use csrf::{CsrfError, CsrfToken, CsrfValidator, SameSite};
pub use field::{
	BooleanField,
	CharField,
	EmailField,
	ErrorType,
	FieldError,
	FieldResult,
	FormField as Field, // Alias for compatibility
	FormField,
	IntegerField,
	Widget,
};
pub use fields::{
	ChoiceField, ComboField, DateField, DateTimeField, DecimalField, DurationField, FileField,
	FloatField, GenericIPAddressField, IPProtocol, ImageField, JSONField, ModelChoiceField,
	ModelMultipleChoiceField, MultiValueField, MultipleChoiceField, RegexField, SlugField,
	SplitDateTimeField, TimeField, URLField, UUIDField,
};
pub use file_handling::{
	FileUploadError, FileUploadHandler, MemoryFileUpload, TemporaryFileUpload,
};
pub use form::{Form, FormError, FormResult};
pub use formset::FormSet;
pub use formsets::{
	FormSetFactory,
	InlineFormSet,
	ModelFormSet as AdvancedModelFormSet, // Renamed to avoid conflict
};
pub use media::{Media, MediaDefiningWidget};
pub use model_form::{FieldType, FormModel, ModelForm, ModelFormBuilder, ModelFormConfig};
pub use model_formset::{ModelFormSet, ModelFormSetBuilder, ModelFormSetConfig};
pub use security::{FormSecurityMiddleware, HoneypotField, RateLimiter, SecurityError};
pub use wizard::{FormWizard, WizardStep};
pub use xss_protection::{XssConfig, XssError, XssProtector};
