//! Core framework type re-exports.

#[cfg(all(feature = "core", native))]
pub use reinhardt_core::{
	endpoint::EndpointMetadata,
	exception::{Error, Result},
};

#[cfg(all(feature = "core", native))]
pub use reinhardt_http::{Handler, Middleware, MiddlewareChain, Request, Response, ViewResult};

#[cfg(all(feature = "core", native))]
pub use reinhardt_http::Extensions;

#[cfg(native)]
pub use hyper::{Method, StatusCode};

#[cfg(all(feature = "core", native))]
pub use reinhardt_core::signals::{
	M2MAction, M2MChangeEvent, Signal, m2m_changed, post_delete, post_save, pre_delete, pre_save,
};

#[cfg(all(feature = "core", native))]
pub use reinhardt_core::validators::{
	CreditCardValidator, EmailValidator, IBANValidator, IPAddressValidator, PhoneNumberValidator,
	UrlValidator, Validate, ValidationError as ValidatorError, ValidationErrors, ValidationResult,
	Validator,
};
