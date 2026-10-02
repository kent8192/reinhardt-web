# Named ClientForm views

Use `form!` to render a named DTO's existing `FormServerMutation`. The DTO defines
the editable fields, declaration order, serialized names, and validation. The
view supplies presentation settings and shares the mutation's runtime.

```rust
use reinhardt_pages::{client_form, form, use_form};
use reinhardt_pages::reactive::ReactiveScope;
use reinhardt_pages::server_fn::{server_fn, ServerFnError};
use reinhardt_core::validators::Validate;
use serde::{Deserialize, Serialize};

#[client_form(server_fn = login, validate)]
#[derive(Clone, Serialize, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(length(min = 1, max = 150))]
    pub username: String,
    #[validate(length(min = 1, max = 128))]
    pub password: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub token: String,
}

#[server_fn]
pub async fn login(request: crate::LoginRequest) -> Result<AuthResponse, ServerFnError> {
    let token = request.username;
    Ok(AuthResponse { token })
}

fn main() {
    ReactiveScope::run(|| {
        let definition = LoginRequestClientForm::new();
        let runtime = use_form(&definition).build();
        let mutation = definition.server_mutation(&runtime).build();

        let page = form! {
            client_form: LoginRequestClientForm,
            mutation: &mutation,
            id: "login",
            styling: { class: "auth-form" },
            customize: {
                username: { label: "Username", autocomplete: "username" },
                password: {
                    widget: PasswordInput,
                    label: "Password",
                    autocomplete: "current-password",
                    help_text: "Enter your account password.",
                },
            },
            submit: { label: "Sign in", pending_label: "Signing in..." },
            summary: { label: "Please correct the following errors" },
        }.into_page();

        assert_eq!(runtime.get_values().username, "");
        assert_eq!(page.render_to_string().matches("name=\"username\"").count(), 1);
    });
}
```

The server function above is a transport example; implement authentication in
the existing application endpoint. Configure success/error callbacks, query
invalidation, redirects, and optional reset on the runtime or mutation before
constructing the view. A submit event prevents native navigation and calls that
mutation's `dispatch()` once. Native rendering never executes a request.

## Syntax

`client_form` must be first and `mutation` is required. Remaining entries may
appear in any order. Every presentation expression is evaluated once in source
order. The descriptor has an inherent `into_page()` method and implements
`IntoPage`.

| Entry | Meaning |
| --- | --- |
| `client_form` | Path to a generated companion, including one from another crate. |
| `mutation` | Reference to a mutation whose form and input match that companion and DTO. |
| `id` | Optional nonempty ID with no ASCII whitespace. |
| `styling` | Form and shared field classes. |
| `customize` | Partial map keyed by exposed Rust field identifiers. |
| `submit` | `label`, `pending_label`, and `class`. |
| `summary` | Validation summary `label`. |

All exposed fields appear in DTO declaration order. Serde names are used for
control `name` attributes and server error resolution. Raw identifiers are
accepted in customization; default labels omit the `r#` prefix. Skipped fields
keep their existing request/default behavior and cannot be customized.

Unknown fields, duplicate properties, incompatible mutations/widgets, and
unsupported presentation properties fail compilation. This mode rejects
`fields`, `action`, `method`, server function declarations, validation rules,
state declarations, and lifecycle callback declarations.

## Fields and widgets

| DTO field | Default | Other supported widgets |
| --- | --- | --- |
| `String`, `Option<String>` | `TextInput` | `Textarea`, `EmailInput`, `UrlInput`, `PasswordInput` |
| Primitive number, optional primitive number | `NumberInput` | None |
| `bool` | `CheckboxInput` | None |
| `Option<bool>` | `Select` | None |
| Enum or optional enum implementing `ClientFormChoiceSource` | `Select` | None |

Optional boolean choices distinguish unset, false, and true. Optional enum
choices distinguish unset from every enum value, including a variant serialized
as an empty string. DOM tokens are `none` and `choice:N`; the transmitted DTO
retains its ordinary serde representation. Invalid DOM choice tokens are ignored.

Optional numeric empty input writes `None`. Invalid numeric edits retain the
last typed value and expose the existing parse error through form validation.
All numeric controls use `step="any"`, leaving integer/range rules to the typed
runtime. Optional strings retain ClientForm's existing trim-and-empty-to-None
request conversion.

Field overrides include `widget`, `label`, `aria_label`, `aria_describedby`,
`help_text`, `autocomplete`, `class`, `wrapper_class`, `label_class`,
`help_class`, and `error_class`. `placeholder` is available for text and numeric
fields. `empty_label` is available only for optional choices; `true_label` and
`false_label` are available only for optional booleans.

Nested fields, collections, uploads, radio layouts, custom widgets, and
ModelForm rendering are outside this mode.

## Styling and validation

| Setting | Default |
| --- | --- |
| `styling.class` | `reinhardt-form` |
| `styling.field_class` | `reinhardt-field` |
| `styling.input_class` | `reinhardt-input` |
| `styling.label_class` | `reinhardt-label` |
| `styling.help_class` | `reinhardt-help` |
| `styling.error_class` | `reinhardt-error` |
| `styling.summary_class` | `reinhardt-validation-summary` |
| `submit.class` | `reinhardt-submit` |

A field override replaces its corresponding shared class, including an explicit
empty string. Submit labels default to `Submit` / `Submitting...`; the summary
label defaults to `Please correct the following errors`. Boolean labels default
to `False` / `True`, and the unset choice label is empty.

DTO validation remains opt-in through `#[client_form(validate)]`. A positive
literal string length minimum on a nonoptional `String` produces `required`
and `aria-required`. Other types and manual validators do not imply a required
checkbox or other constraint.

Forms use `novalidate` and do not project or accept `min`, `max`, `minlength`,
`maxlength`, or `pattern` overrides. DTO string validation counts Unicode scalar
values, while HTML length limits count UTF-16 code units. Runtime validation
therefore remains authoritative, including for supplementary-plane characters.

## Accessibility, hydration, and reset

Labels, controls, help/error containers, the summary region, and the submit
button retain their nodes while values, validation, or pending state change.
Only the button is disabled while submitting. Controls keep focus and selection.

The summary contains field errors in declaration order followed by nonempty
form/submit errors. Only equal global messages are deduplicated. Unmatched
server field names remain the runtime's existing form-level message. Summary
links focus their retained controls; ordinary updates never move focus.

Generated help/error IDs are combined with external `aria_describedby` IDs
without duplicates. One summary live region announces errors. Explicit
`aria_label` overrides the accessible name while retaining the visible label.

Without an explicit ID, the view calls `use_id_with_prefix("client-form")` once.
SSR and hydration must use the same ID scope and construction order. Explicit
IDs are useful for independently rendered roots. IDs for fields, help, and
errors use declaration ordinals, so serde aliases do not create collisions.

Hydration reuses the existing typed bindings, preserves browser edits made
before hydration, and omits password values from SSR. Explicit runtime field
writes and resets take precedence over stale browser state. Native form reset
uses the shared generated-control reset coordinator and performs runtime
bookkeeping once per batch. Disposed scopes do not receive late reset or
submission callbacks.
