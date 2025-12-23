//! # Reinhardt Forms
//!
//! Form processing and validation for the Reinhardt framework.
//!
//! ## Overview
//!
//! This crate provides comprehensive form processing capabilities inspired by Django's
//! form system, including data validation, CSRF protection, file uploads, and multi-step
//! form wizards.
//!
//! ## Features
//!
//! - **[`Form`]**: Base form class with validation and rendering
//! - **[`ModelForm`]**: Auto-generated forms from model definitions
//! - **[`FormSet`]**: Handle multiple forms of the same type
//! - **[`FormWizard`]**: Multi-step form workflows
//! - **Field Types**: 20+ field types (CharField, IntegerField, EmailField, etc.)
//! - **CSRF Protection**: Cryptographic token generation with SameSite cookie support
//! - **File Handling**: Upload with MIME detection, chunked uploads, and progress tracking
//! - **Security**: XSS protection, honeypot fields, and rate limiting
//!
//! ## Quick Start
//!
//! ### Basic Form
//!
//! ```rust,ignore
//! use reinhardt_forms::{Form, CharField, EmailField, IntegerField};
//!
//! #[derive(Form)]
//! struct ContactForm {
//!     name: CharField,
//!     email: EmailField,
//!     age: IntegerField,
//!     message: CharField,
//! }
//!
//! // Validate form data
//! let form = ContactForm::from_data(&request_data);
//! if form.is_valid() {
//!     let name = form.cleaned_data.name;
//!     // Process the form...
//! } else {
//!     let errors = form.errors();
//! }
//! ```
//!
//! ### Model Form
//!
//! ```rust,ignore
//! use reinhardt_forms::{ModelForm, ModelFormBuilder};
//!
//! // Auto-generate form from User model
//! let form = ModelFormBuilder::<User>::new()
//!     .fields(&["username", "email", "bio"])
//!     .exclude(&["password"])
//!     .build();
//! ```
//!
//! ## Available Field Types
//!
//! | Field | Description |
//! |-------|-------------|
//! | [`CharField`] | Text input with max_length validation |
//! | [`IntegerField`] | Integer input with min/max validation |
//! | [`FloatField`] | Floating-point number input |
//! | [`DecimalField`] | Decimal number with precision control |
//! | [`BooleanField`] | Checkbox input |
//! | [`EmailField`] | Email address validation |
//! | [`URLField`] | URL validation |
//! | [`DateField`] | Date input with format parsing |
//! | [`DateTimeField`] | DateTime input |
//! | [`TimeField`] | Time input |
//! | [`DurationField`] | Duration input |
//! | [`FileField`] | File upload |
//! | [`ImageField`] | Image upload with dimension validation |
//! | [`ChoiceField`] | Select dropdown |
//! | [`MultipleChoiceField`] | Multi-select |
//! | [`ModelChoiceField`] | Foreign key selection |
//! | [`JSONField`] | JSON data input |
//! | [`UUIDField`] | UUID input |
//! | [`SlugField`] | URL-safe slug input |
//! | [`RegexField`] | Custom regex validation |
//!
//! ## CSRF Protection
//!
//! ```rust,ignore
//! use reinhardt_forms::{CsrfToken, CsrfValidator, SameSite};
//!
//! // Generate a CSRF token
//! let token = CsrfToken::generate();
//!
//! // Validate token from request
//! let validator = CsrfValidator::new()
//!     .same_site(SameSite::Strict)
//!     .check_origin(true);
//!
//! if validator.validate(&request, &token).is_ok() {
//!     // Token is valid
//! }
//! ```
//!
//! ## File Uploads
//!
//! ```rust,ignore
//! use reinhardt_forms::{FileUploadHandler, ChunkedUploadManager};
//!
//! // Handle file upload
//! let handler = FileUploadHandler::new()
//!     .max_size(10 * 1024 * 1024)  // 10MB
//!     .allowed_types(&["image/jpeg", "image/png"]);
//!
//! let file = handler.process(&request).await?;
//!
//! // Chunked upload with progress
//! let manager = ChunkedUploadManager::new();
//! let session = manager.start_upload("large_file.zip", total_size).await?;
//!
//! for chunk in chunks {
//!     let progress = session.upload_chunk(chunk).await?;
//!     println!("Progress: {}%", progress.percentage);
//! }
//! ```
//!
//! ## FormSets
//!
//! Handle multiple forms of the same type:
//!
//! ```rust,ignore
//! use reinhardt_forms::{FormSet, FormSetFactory};
//!
//! // Create a formset with 3 forms
//! let formset = FormSetFactory::<ItemForm>::new()
//!     .extra(3)
//!     .min_num(1)
//!     .max_num(10)
//!     .build();
//!
//! if formset.is_valid() {
//!     for form in formset.forms() {
//!         // Process each form
//!     }
//! }
//! ```
//!
//! ## Form Wizard
//!
//! Multi-step forms:
//!
//! ```rust,ignore
//! use reinhardt_forms::{FormWizard, WizardStep};
//!
//! let wizard = FormWizard::new()
//!     .add_step(WizardStep::new("account", AccountForm::new()))
//!     .add_step(WizardStep::new("profile", ProfileForm::new()))
//!     .add_step(WizardStep::new("confirmation", ConfirmForm::new()));
//!
//! // Process wizard step
//! let result = wizard.process_step(&request).await?;
//! ```
//!
//! ## Security Features
//!
//! ```rust,ignore
//! use reinhardt_forms::{XssProtector, HoneypotField, RateLimiter};
//!
//! // XSS protection
//! let protector = XssProtector::new();
//! let safe_html = protector.sanitize(user_input);
//!
//! // Honeypot field (spam protection)
//! let honeypot = HoneypotField::new("website");  // Hidden field
//! if !honeypot.is_empty(&form_data) {
//!     // Likely a bot submission
//! }
//!
//! // Rate limiting
//! let limiter = RateLimiter::new(10, Duration::from_secs(60));  // 10 per minute
//! if limiter.check(&client_ip).is_err() {
//!     // Rate limit exceeded
//! }
//! ```

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
pub mod wasm_compat; // Week 5 Day 1: WASM compatibility layer
pub mod wizard;
pub mod xss_protection;

pub use bound_field::BoundField;
pub use chunked_upload::{
	ChunkedUploadError, ChunkedUploadManager, ChunkedUploadSession, UploadProgress,
};
pub use csrf::{CsrfError, CsrfToken, CsrfValidator, SameSite};
pub use field::{
	ErrorType,
	FieldError,
	FieldResult,
	FormField as Field, // Alias for compatibility
	FormField,
	Widget,
};
pub use fields::{
	BooleanField, CharField, ChoiceField, ComboField, DateField, DateTimeField, DecimalField,
	DurationField, EmailField, FileField, FloatField, GenericIPAddressField, IPProtocol,
	ImageField, IntegerField, JSONField, ModelChoiceField, ModelMultipleChoiceField,
	MultiValueField, MultipleChoiceField, RegexField, SlugField, SplitDateTimeField, TimeField,
	URLField, UUIDField,
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
