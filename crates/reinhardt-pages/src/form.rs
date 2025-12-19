//! Form Integration for Reinhardt WASM (Week 5 Day 3-4)
//!
//! This module provides client-side form rendering and handling
//! integrated with `reinhardt-forms` server-side forms.
//!
//! ## Architecture
//!
//! ```text
//! Server-side:                 Client-side (WASM):
//! ┌──────────────┐            ┌─────────────────┐
//! │reinhardt-    │            │reinhardt-pages   │
//! │forms         │            │                 │
//! │              │            │                 │
//! │ Form         │            │ FormComponent   │
//! │ │to_metadata()│────JSON───▶│ │render()      │──▶ DOM
//! │ │            │            │ │validate()    │
//! │ │            │            │ │submit()      │──▶ AJAX
//! │ │            │            │                 │
//! │ FormMetadata │            │ FormBinding<F>  │
//! │              │            │ │bind()        │──▶ Signals
//! └──────────────┘            └─────────────────┘
//! ```
//!
//! ## Components
//!
//! - **FormComponent**: Renders `FormMetadata` to DOM with CSRF protection
//! - **FormBinding**: Two-way data binding between Form and Signals
//!
//! ## Example
//!
//! ```ignore
//! use reinhardt_pages::form::FormComponent;
//! use reinhardt_forms::wasm_compat::FormMetadata;
//!
//! // Receive FormMetadata from server
//! let metadata: FormMetadata = fetch_form_metadata().await?;
//!
//! // Create and render form
//! let form = FormComponent::new(metadata, "/api/submit");
//! let form_element = form.render();
//! document.body().append_child(&form_element)?;
//!
//! // Submit on user action
//! if form.validate() {
//!     form.submit().await?;
//! }
//! ```

pub mod binding;
pub mod component;

pub use binding::FormBinding;
pub use component::FormComponent;
