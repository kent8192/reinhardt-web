//! AST definitions for the page! and form! macro DSLs.
//!
//! This crate provides the Abstract Syntax Tree (AST) structures and parsing logic
//! for the `page!` and `form!` macros' Domain Specific Languages (DSLs). It is designed
//! to be shared between the proc-macro crate and other tools like formatters.
//!
//! ## page! DSL Structure
//!
//! ```text
//! // Basic structure
//! page!(|props| {
//!     element {
//!         attr: value,
//!         @event: handler,
//!         child_element { ... }
//!         "text content"
//!         { expression }
//!     }
//! })
//!
//! // With head directive (for SSR)
//! page! {
//!     #head: head_expr,
//!     |props| {
//!         element { ... }
//!     }
//! }
//! ```
//!
//! ## form! DSL Structure
//!
//! ```text
//! form! {
//!     name: LoginForm,
//!     action: "/api/login",    // OR server_fn: submit_login
//!     method: Post,
//!     class: "form-container",
//!
//!     fields: {
//!         username: CharField {
//!             required,
//!             max_length: 150,
//!             label: "Username",
//!             class: "input-field",
//!         },
//!         password: CharField {
//!             required,
//!             widget: PasswordInput,
//!         },
//!     },
//!
//!     validators: {
//!         username: [|v| !v.is_empty() => "Required"],
//!     },
//! }
//! ```
//!
//! ## Main Types
//!
//! ### page! Untyped AST (from parser)
//!
//! - [`PageMacro`] - The top-level AST node representing the entire macro invocation
//! - [`PageNode`] - A node in the view tree (element, text, expression, control flow, component)
//! - [`PageElement`] - An HTML element with attributes, events, and children
//! - [`PageAttr`] - An attribute on an element
//! - [`PageEvent`] - An event handler on an element
//! - [`PageIf`] - Conditional rendering
//! - [`PageFor`] - List rendering
//! - [`PageWatch`] - Reactive watch block for Signal-dependent expressions
//! - [`PageComponent`] - A component call
//!
//! ### page! Typed AST (from validator)
//!
//! - [`TypedPageMacro`] - Validated and typed AST with type-safe attributes
//! - [`TypedPageNode`] - Typed nodes with validated attribute values
//! - [`TypedPageElement`] - Element with typed attributes
//! - [`TypedPageAttr`] - Attribute with typed value
//! - [`TypedPageWatch`] - Reactive watch block for Signal-dependent expressions
//! - `AttrValue` - Typed representation of attribute values
//!
//! ### form! Untyped AST (from parser)
//!
//! - [`FormMacro`] - The top-level form AST node
//! - [`FormFieldDef`] - A field definition with type and properties
//! - [`FormFieldProperty`] - A property within a field (named, flag, widget)
//! - [`FormValidator`] - Server-side validator definition
//! - [`ClientValidator`] - Client-side validator definition
//!
//! ### form! Typed AST (from validator)
//!
//! - [`TypedFormMacro`] - Validated form with typed fields
//! - [`TypedFormFieldDef`] - Validated field with typed properties
//! - [`TypedFieldType`] - Validated field type with Signal mapping
//! - [`TypedWidget`] - Validated widget type
//! - [`TypedFieldStyling`] - Styling properties with defaults
//!
//! ## Usage
//!
//! ```rust,ignore
//! use reinhardt_pages::ast::PageMacro;
//! use syn::parse2;
//! use quote::quote;
//!
//! let tokens = quote! {
//!     |name: String| {
//!         div {
//!             class: "container",
//!             h1 { "Hello, " name }
//!         }
//!     }
//! };
//!
//! let page_macro: PageMacro = parse2(tokens).unwrap();
//! ```

mod node;
mod parser;
pub mod typed_node;
pub mod types;

// Form macro AST
pub mod form_node;
mod form_parser;
pub mod form_typed;

pub use node::{
	PageAttr, PageBody, PageComponent, PageComponentArg, PageElement, PageElse, PageEvent,
	PageExpression, PageFor, PageIf, PageMacro, PageNode, PageParam, PageText, PageWatch,
	debug_tokens,
};
pub use typed_node::{
	TypedPageAttr, TypedPageBody, TypedPageComponent, TypedPageElement, TypedPageElse,
	TypedPageFor, TypedPageIf, TypedPageMacro, TypedPageNode, TypedPageWatch,
};

// Form macro exports
pub use form_node::{
	ClientValidator, ClientValidatorRule, CustomAttr, FormAction, FormCallbacks, FormDerived,
	FormDerivedItem, FormFieldDef, FormFieldEntry, FormFieldGroup, FormFieldProperty, FormMacro,
	FormSlots, FormState, FormStateField, FormValidator, FormWatch, FormWatchItem, IconAttr,
	IconChild, IconElement, IconPosition, ValidatorRule, WrapperAttr, WrapperElement,
};
pub use form_typed::{
	FormMethod, TypedChoicesConfig, TypedClientValidator, TypedClientValidatorRule,
	TypedCustomAttr, TypedDerivedItem, TypedFieldDisplay, TypedFieldStyling, TypedFieldType,
	TypedFieldValidation, TypedFormAction, TypedFormCallbacks, TypedFormDerived, TypedFormFieldDef,
	TypedFormFieldEntry, TypedFormFieldGroup, TypedFormMacro, TypedFormSlots, TypedFormState,
	TypedFormStyling, TypedFormValidator, TypedFormWatch, TypedFormWatchItem, TypedIcon,
	TypedIconAttr, TypedIconChild, TypedIconPosition, TypedValidatorRule, TypedWidget,
	TypedWrapper, TypedWrapperAttr,
};
