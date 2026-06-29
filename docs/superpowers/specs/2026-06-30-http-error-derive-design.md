# HttpError Derive Macro Design

## Summary

Add a `#[derive(HttpError)]` procedural macro and a small runtime trait that let application-defined error enums bind each variant to an HTTP status code and a client-facing message. The design removes repetitive route-level `match` blocks while preserving Reinhardt's existing safe error response behavior by default.

## Goals

- Let application error enums declare HTTP status and client message mapping at the variant definition.
- Require every enum variant to declare an explicit status mapping.
- Support both fixed client messages and delegation to existing application methods.
- Keep response conversion opt-in so deriving the trait does not automatically change runtime behavior.
- Reuse `SafeErrorResponse` by default to avoid exposing internal details from server errors.

## Non-Goals

- Do not generate the original error enum.
- Do not infer status codes from variant names.
- Do not add middleware-level exception handling for arbitrary application errors.
- Do not replace `reinhardt_core::exception::Error`; this feature is for user-defined error types.

## Public API

Add a runtime trait under `reinhardt_core::exception`:

```rust
pub use http::StatusCode;

pub trait HttpError {
    fn status_code(&self) -> StatusCode;
    fn client_message(&self) -> std::borrow::Cow<'static, str>;
}
```

The root facade re-exports both the trait and derive macro:

```rust
pub use reinhardt_core::exception::HttpError;
pub use reinhardt_macros::HttpError;
```

The trait lives in `reinhardt-core` because it only depends on `http::StatusCode` and standard library types. `StatusCode` is re-exported from `reinhardt_core::exception` so macro expansion does not require application crates to depend on `http` directly. Response conversion remains generated code in the user's crate, so it can implement `From<MyError> for Response` without violating Rust's orphan rules.

## Macro Syntax

The derive macro applies to enums only:

```rust
use reinhardt::{HttpError, StatusCode};

#[derive(Debug, thiserror::Error, HttpError)]
#[http_error(response, body = "error")]
enum WritingError {
    #[error("validation error: {0}")]
    #[http_error(status = BAD_REQUEST, message = "Invalid request")]
    Validation(String),

    #[error("not found: {0}")]
    #[http_error(status = NOT_FOUND, message_fn = client_message)]
    NotFound(String),
}
```

Variant attributes:

- `status = BAD_REQUEST` is required for every variant. The macro expands this through the resolved Reinhardt crate path, such as `::reinhardt::StatusCode::BAD_REQUEST` for facade users or `::reinhardt_core::exception::StatusCode::BAD_REQUEST` for direct `reinhardt-core` users.
- `message = "..."` returns a fixed client-facing message as `Cow::Borrowed`.
- `message_fn = client_message` delegates to an existing method on the error type and converts the result with `Into<Cow<'static, str>>`.
- `message` and `message_fn` are mutually exclusive.

Enum attributes:

- `#[http_error(response)]` enables `From<ErrorType> for Response` generation.
- `body = "safe"` uses `SafeErrorResponse` and is the default response body mode.
- `body = "error"` produces `{"error": client_message}` for application APIs that want that exact envelope.

Generated `client_message()` code uses `#[deny(unconditional_recursion)]` so a missing or misresolved `message_fn` becomes a compile error instead of silently recursing.

## Response Conversion

When `#[http_error(response)]` is present, the derive macro emits:

```rust
impl From<WritingError> for reinhardt::Response {
    fn from(error: WritingError) -> Self {
        // generated body
    }
}
```

For `body = "safe"`, generated code builds a `SafeErrorResponse`:

- The status comes from `HttpError::status_code(&error)`.
- For 4xx statuses, `client_message()` is added as safe detail.
- For 5xx statuses, detail is omitted by default, matching the existing `From<reinhardt::Error> for Response` behavior.

For `body = "error"`, generated code returns JSON with the application message:

```json
{"error":"Invalid request"}
```

This mode intentionally trusts the application-defined `client_message()` value. Documentation must state that messages used with `body = "error"` must be safe for clients, including for 5xx responses.

Route code can then remove manual status mapping:

```rust
let results = search.search_sources(request).await.map_err(Response::from)?;
json_response(StatusCode::OK, &results)
```

## Macro Validation

The macro reports compile errors for:

- Applying `#[derive(HttpError)]` to a non-enum item.
- Missing `#[http_error(status = ...)]` on any variant.
- Missing both `message` and `message_fn` on any variant.
- Specifying both `message` and `message_fn`.
- Unknown enum-level or variant-level `http_error` keys.
- Unsupported `body` values.

The initial syntax only supports unit, tuple, and struct variants by matching with `..` where needed. The mapping does not inspect fields.

## Tests

Unit tests in `reinhardt-macros` cover attribute parsing:

- Required status.
- Mutually exclusive message sources.
- `response` and `body` parsing.
- Unknown attributes and unsupported body modes.

Trybuild tests cover compile-time behavior:

- Pass cases for fixed messages, method messages, and response conversion.
- Fail cases for non-enum derive, missing status, missing message source, duplicate message source, unknown keys, and invalid body mode.
- A fail case that catches `message_fn` resolving to unconditional recursion.

Runtime tests cover response behavior:

- `body = "safe"` maps status correctly.
- `body = "safe"` includes detail for 4xx responses.
- `body = "safe"` omits detail for 5xx responses.
- `body = "error"` returns the exact `{"error": ...}` envelope.

## Documentation

Update these documentation surfaces with the derive-based pattern:

- `reinhardt-core` exception docs for the `HttpError` trait.
- Root README/API docs for application error response mapping.
- The Axum migration guide error handling section, keeping the manual `From<AppError> for Response` form as a lower-level alternative.

Docs must describe client-message safety without referencing the origin of the request or implementation discussion.
