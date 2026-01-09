# Part 4: Client-Side Forms and Component Patterns

In this tutorial, we'll implement form handling in reinhardt-pages using client-side components and server functions.

## Understanding Form Handling in reinhardt-pages

Unlike traditional server-rendered forms (using templates like Tera), reinhardt-pages handles forms on the client side with WASM components that communicate with server functions.

> **📌 Important Note**: This tutorial demonstrates two approaches to form handling:
> 
> 1. **`form!` Macro Approach (Recommended)**: The example implementation in `src/client/components/polls.rs` uses the declarative `form!` macro, which automatically handles:
>    - Form state management with reactive `watch` blocks
>    - Dynamic choices population
>    - Automatic CSRF token injection for POST/PUT/PATCH/DELETE methods
>    - Loading state management
>    - Error display
> 
> 2. **Manual Form Handling (Alternative)**: The manual approach documented below provides fine-grained control but requires more boilerplate code.
> 
> For most use cases, **we recommend using the `form!` macro** as demonstrated in the actual implementation. See `src/client/components/polls.rs` for a complete example of the `form!` macro pattern with dynamic choices.

**Key Concepts:**

1. **Declarative Forms**: Use `form!` macro for automatic state management and CSRF protection
2. **Manual Forms**: Use `use_state()` to manually manage form data when needed
3. **Server Functions**: Call server functions for data persistence and validation
4. **Error Handling**: Display validation errors and server errors to users
5. **Navigation**: Client-side navigation after successful form submission

## Example: Declarative Forms with form! Macro (Recommended)

The voting form in `src/client/components/polls.rs` demonstrates the recommended approach:

```rust
// Example from src/client/components/polls.rs
let voting_form = form! {
	name: VotingForm,
	server_fn: submit_vote,
	method: Post,
	state: { loading, error },

	fields: {
		question_id: HiddenField {
			initial: question_id_str.clone(),
		},
		choice_id: ChoiceField {
			widget: RadioSelect,
			required,
			label: "Select your choice",
			class: "form-check poll-choice p-3 mb-2 border rounded",
			choices_from: "choices",  // Dynamic choices
			choice_value: "id",
			choice_label: "choice_text",
		},
	},

	watch: {
		submit_button: |form| {
			let is_loading = form.loading().get();
			// Reactive UI updates when loading state changes
			page!(|is_loading: bool| {
				button {
					r#type: "submit",
					class: if is_loading { "btn btn-primary disabled" } else { "btn btn-primary" },
					disabled: is_loading,
					{ if is_loading { "Voting..." } else { "Vote" } }
				}
			})(is_loading)
		},
		error_display: |form| {
			let err = form.error().get();
			// Show error messages reactively
			page!(|err: Option<String>| {
				watch {
					if let Some(e) = err.clone() {
						div { class: "alert alert-danger mt-3", { e } }
					}
				}
			})(err)
		},
	},

	on_success: |_result| {
		// Navigate after successful submission
		#[cfg(target_arch = "wasm32")]
		{
			if let Some(window) = web_sys::window() {
				let _ = window.location().set_href(&format!("/polls/{}/results/", question_id));
			}
		}
	},
};

// Populate choices dynamically from server
#[cfg(target_arch = "wasm32")]
{
	spawn_local(async move {
		match get_question_detail(question_id).await {
			Ok((q, choices)) => {
				let choice_options: Vec<(String, String)> = choices
					.iter()
					.map(|c| (c.id.to_string(), c.choice_text.clone()))
					.collect();
				voting_form.choice_id_choices().set(choice_options);
			}
			Err(e) => { /* handle error */ }
		}
	});
}
```

**Key Benefits:**
- **Automatic CSRF Protection**: POST method automatically includes CSRF token
- **Reactive State Management**: `watch` blocks update UI when form state changes
- **Dynamic Choices**: `choices_from` populates select/radio options at runtime
- **Built-in Loading States**: `form.loading()` and `form.error()` Signals
- **Type-Safe**: Compiler ensures field names match form definition

For complete implementation, see `examples/local/examples-tutorial-basis/src/client/components/polls.rs`.

## Manual Form Handling (Alternative Approach)

Let's implement the voting functionality. We already created the form structure in Part 3, but now we'll add proper state management and error handling.

### Update the Detail Page Component

Update `src/client/components/polls.rs` to add comprehensive form handling:

```rust
// src/client/components/polls.rs
use reinhardt_pages::prelude::*;
use crate::shared::types::{ChoiceInfo, QuestionInfo, VoteRequest};
use crate::server_fn::polls::{get_question_detail, vote};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local;

pub fn polls_detail_page(question_id: i64) -> View {
	// State management
	let (question, set_question) = use_state(None::<QuestionInfo>);
	let (choices, set_choices) = use_state(Vec::<ChoiceInfo>::new());
	let (loading, set_loading) = use_state(true);
	let (error, set_error) = use_state(None::<String>);

	// Form state
	let (selected_choice, set_selected_choice) = use_state(None::<i64>);
	let (voting, set_voting) = use_state(false);
	let (form_error, set_form_error) = use_state(None::<String>);

	// Load question and choices on mount
	#[cfg(target_arch = "wasm32")]
	{
		let set_question = set_question.clone();
		let set_choices = set_choices.clone();
		let set_loading = set_loading.clone();
		let set_error = set_error.clone();

		spawn_local(async move {
			match get_question_detail(question_id).await {
				Ok((q, c)) => {
					set_question(Some(q));
					set_choices(c);
					set_loading(false);
				}
				Err(e) => {
					set_error(Some(e.to_string()));
					set_loading(false);
				}
			}
		});
	}

	// Handle choice selection
	let handle_choice_change = {
		let set_selected_choice = set_selected_choice.clone();
		let set_form_error = set_form_error.clone();

		move |e: web_sys::Event| {
			if let Some(target) = e.target() {
				if let Some(input) = target.dyn_ref::<web_sys::HtmlInputElement>() {
					if let Ok(choice_id) = input.value().parse::<i64>() {
						set_selected_choice(Some(choice_id));
						set_form_error(None); // Clear error when user selects
					}
				}
			}
		}
	};

	// Handle form submission
	let handle_submit = {
		let selected_choice = selected_choice.clone();
		let set_voting = set_voting.clone();
		let set_form_error = set_form_error.clone();
		let question_id = question_id;

		move |e: web_sys::Event| {
			e.prevent_default();

			// Client-side validation
			let selected = selected_choice.get().clone();
			if selected.is_none() {
				set_form_error(Some("Please select a choice.".to_string()));
				return;
			}

			let choice_id = selected.unwrap();
			let set_voting = set_voting.clone();
			let set_form_error = set_form_error.clone();

			// Submit to server
			#[cfg(target_arch = "wasm32")]
			spawn_local(async move {
				set_voting(true);

				match vote(VoteRequest { question_id, choice_id }).await {
					Ok(_) => {
						// Navigate to results page
						if let Some(window) = web_sys::window() {
							if let Ok(location) = window.location() {
								let _ = location.set_href(&format!("/polls/{}/results/", question_id));
							}
						}
					}
					Err(e) => {
						set_form_error(Some(format!("Vote failed: {}", e)));
						set_voting(false);
					}
				}
			});
		}
	};

	// Get current state for rendering
	let question_data = question.get();
	let choices_data = choices.get();
	let loading_state = loading.get();
	let error_state = error.get();
	let form_error_state = form_error.get();
	let voting_state = voting.get();
	let selected = selected_choice.get();

	page!(|
		question_data: Option<QuestionInfo>,
		choices_data: Vec<ChoiceInfo>,
		loading_state: bool,
		error_state: Option<String>,
		form_error_state: Option<String>,
		voting_state: bool,
		selected: Option<i64>,
		handle_submit: impl Fn(web_sys::Event) + 'static,
		handle_choice_change: impl Fn(web_sys::Event) + 'static
	| {
		div {
			class: "container mt-5",

			if let Some(ref err) = error_state {
				div {
					class: "alert alert-danger",
					{err}
				}
			} else if loading_state {
				div {
					class: "spinner-border text-primary",
					role: "status",
					span {
						class: "visually-hidden",
						"Loading..."
					}
				}
			} else if let Some(ref q) = question_data {
				div {
					h1 { class: "mb-4", {&q.question_text} }

					if let Some(ref form_err) = form_error_state {
						div {
							class: "alert alert-warning",
							{form_err}
						}
					}

					form {
						onsubmit: handle_submit,

						div {
							class: "mb-3",
							for choice in &choices_data {
								div {
									class: "form-check",
									input {
										class: "form-check-input",
										type: "radio",
										name: "choice",
										id: format!("choice{}", choice.id),
										value: choice.id.to_string(),
										onchange: handle_choice_change.clone(),
										checked: selected == Some(choice.id)
									}
									label {
										class: "form-check-label",
										for: format!("choice{}", choice.id),
										{&choice.choice_text}
									}
								}
							}
						}

						button {
							class: "btn btn-primary",
							type: "submit",
							disabled: voting_state,
							if voting_state {
								"Voting..."
							} else {
								"Vote"
							}
						}

						" "
						a {
							href: format!("/polls/{}/results/", q.id),
							class: "btn btn-secondary",
							"View Results"
						}
					}
				}
			} else {
				div {
					class: "alert alert-info",
					"Question not found"
				}
			}

			div {
				class: "mt-3",
				a {
					href: "/",
					class: "btn btn-link",
					"← Back to Polls"
				}
			}
		}
	})(
		question_data,
		choices_data,
		loading_state,
		error_state,
		form_error_state,
		voting_state,
		selected,
		handle_submit,
		handle_choice_change
	)
}
```

**Key Features:**

1. **Form State Management**:
   - `selected_choice` - Currently selected radio button
   - `voting` - Loading state during submission
   - `form_error` - Client-side validation errors

2. **Event Handlers**:
   - `handle_choice_change` - Updates selected choice when user clicks radio button
   - `handle_submit` - Validates and submits form

3. **Client-Side Validation**:
   - Check if a choice is selected before submission
   - Clear error messages when user interacts with form

4. **Error Display**:
   - Show form validation errors
   - Show server errors from failed submissions
   - Different styling for different error types

## Server-Side Validation and Processing

The server function handles data persistence and server-side validation. Update `src/server_fn/polls.rs`:

```rust
// src/server_fn/polls.rs
use crate::shared::types::{ChoiceInfo, VoteRequest};

#[cfg(not(target_arch = "wasm32"))]
use reinhardt::pages::server_fn::{server_fn, ServerFnError};
#[cfg(target_arch = "wasm32")]
use reinhardt_pages::server_fn::{server_fn, ServerFnError};

/// Vote for a choice
///
/// Server-side validation and atomic database update.
#[cfg(not(target_arch = "wasm32"))]
#[server_fn(use_inject = true)]
pub async fn vote(
	request: VoteRequest,
	#[inject] _db: reinhardt::DatabaseConnection,
) -> std::result::Result<ChoiceInfo, ServerFnError> {
	use crate::apps::polls::models::Choice;
	use reinhardt::Model;

	let choice_manager = Choice::objects();

	// Server-side validation: Get the choice
	let mut choice = choice_manager
		.get(request.choice_id)
		.first()
		.await
		.map_err(|e| ServerFnError::ServerError(e.to_string()))?
		.ok_or_else(|| ServerFnError::ServerError("Choice not found".to_string()))?;

	// Server-side validation: Verify the choice belongs to the question
	if choice.question_id != request.question_id {
		return Err(ServerFnError::ServerError(
			"Choice does not belong to this question".to_string(),
		));
	}

	// Atomic increment using database-level operation
	// This prevents race conditions
	choice.votes += 1;

	// Update in database
	let updated_choice = choice_manager
		.update(&choice)
		.await
		.map_err(|e| ServerFnError::ServerError(e.to_string()))?;

	Ok(ChoiceInfo::from(updated_choice))
}

#[cfg(target_arch = "wasm32")]
#[server_fn]
pub async fn vote(_request: VoteRequest) -> std::result::Result<ChoiceInfo, ServerFnError> {
	unreachable!()
}
```

**Server-Side Validation Benefits:**

1. **Security**: Never trust client-side validation alone
2. **Data Integrity**: Verify business rules at the server
3. **Consistency**: Centralized validation logic
4. **Error Messages**: Provide detailed feedback to clients

**Race Condition Prevention:**

For atomic updates, use database-level operations:

```rust
// Future enhancement: Use F expressions for atomic updates
use reinhardt::db::orm::F;

Choice::objects()
	.filter(Choice::field_id().eq(choice_id))
	.update()
	.set(Choice::field_votes(), F::new(Choice::field_votes()) + 1)
	.execute(&db)
	.await?;
```

**Why This Prevents Race Conditions:**

1. **Single UPDATE Query**: Database executes the increment atomically
2. **No SELECT Needed**: Avoids read-modify-write race condition
3. **Database Guarantees**: ACID properties ensure consistency

## CSRF Protection in reinhardt-pages

In traditional server-rendered applications, CSRF protection uses tokens embedded in templates:

```html
<!-- Old approach (Tera template) -->
<form method="post">
  {% csrf_token %}
  <!-- form fields -->
</form>
```

In reinhardt-pages, CSRF protection is handled differently:

## CSRF Protection in reinhardt-pages

### Automatic CSRF Protection

reinhardt-pages provides **automatic CSRF protection** for both forms and server functions. No manual token handling is required.

#### For Forms

The `form!` macro automatically injects CSRF tokens for POST/PUT/PATCH/DELETE methods:

```rust
// POST form automatically includes CSRF token
let contact_form = form! {
	name: ContactForm,
	action: "/api/contact",
	method: Post,

	fields: {
		message: CharField { required },
	},
};
```

The generated HTML includes a hidden CSRF token field:

```html
<form action="/api/contact" method="post">
	<input type="hidden" name="csrfmiddlewaretoken" value="[token]">
	<!-- field elements -->
</form>
```

GET forms do NOT include CSRF tokens since they are safe methods that don't need CSRF protection:

```rust
// GET form does NOT include CSRF token (safe method)
let search_form = form! {
	name: SearchForm,
	action: "/search",
	method: Get,

	fields: {
		query: CharField { required },
	},
};
```

#### For Server Functions

The `#[server_fn]` macro automatically includes CSRF headers in all requests by default:

```rust
#[server_fn]  // CSRF protection enabled by default
async fn vote(request: VoteRequest) -> Result<ChoiceInfo, ServerFnError> {
	// CSRF header automatically included: X-CSRFToken: [token]
	// No manual CSRF handling needed
	
	// ... business logic
}
```

**Important**: The `vote` function we created earlier in this tutorial is already CSRF-protected by default. No additional code is needed.

#### Disabling CSRF Protection (Optional)

For public APIs that don't require CSRF protection, you can disable it:

```rust
#[server_fn(no_csrf = true)]  // Disable CSRF for public APIs
async fn public_api() -> Result<Data, ServerFnError> {
	// No CSRF header - useful for public endpoints
	
	// ... business logic
}
```

### Token Retrieval

CSRF tokens are automatically retrieved from multiple sources in the following order of priority:

1. **Cookie**: `csrftoken`
2. **Meta tag**: `<meta name="csrf-token">`
3. **Hidden input**: `<input name="csrfmiddlewaretoken">`

For advanced use cases where you need manual control over CSRF tokens:

```rust
use reinhardt_pages::csrf::{get_csrf_token, csrf_headers};

// Get token (tries Cookie → Meta → Input in that order)
if let Some(token) = get_csrf_token() {
	// Use token for manual AJAX requests
}

// Get as HTTP header (for fetch API)
if let Some((header_name, header_value)) = csrf_headers() {
	// header_name: "X-CSRFToken"
	// header_value: token from browser
}
```

### Django Compatibility

reinhardt-pages uses Django-compatible CSRF conventions:

- **Cookie name**: `csrftoken`
- **Header name**: `X-CSRFToken`
- **Form field name**: `csrfmiddlewaretoken`

This ensures compatibility with Django backends and existing Django infrastructure.

## Component Patterns: Reusability Instead of Generic Views

In traditional server-rendered frameworks, "generic views" provide reusable patterns for common tasks (list views, detail views, etc.). In reinhardt-pages, we achieve similar reusability through **component composition**.

### Pattern 1: Reusable Loading Component

Extract common loading patterns:

```rust
// src/client/components/common.rs
use reinhardt_pages::prelude::*;

pub fn loading_spinner() -> View {
	page!(|| {
		div {
			class: "spinner-border text-primary",
			role: "status",
			span {
				class: "visually-hidden",
				"Loading..."
			}
		}
	})()
}

pub fn error_alert(message: &str) -> View {
	let msg = message.to_string();
	page!(|msg: String| {
		div {
			class: "alert alert-danger",
			{msg}
		}
	})(msg)
}
```

Usage:

```rust
use crate::client::components::common::{loading_spinner, error_alert};

pub fn polls_index() -> View {
	// ... state management

	page!(|loading_state: bool, error_state: Option<String>| {
		div {
			class: "container mt-5",

			if let Some(ref err) = error_state {
				{error_alert(err)}
			} else if loading_state {
				{loading_spinner()}
			} else {
				// ... content
			}
		}
	})(loading_state, error_state)
}
```

### Pattern 2: Form Field Components

Create reusable form field components:

```rust
// src/client/components/forms.rs
use reinhardt_pages::prelude::*;

pub fn radio_choice(
	id: &str,
	name: &str,
	value: &str,
	label: &str,
	checked: bool,
	on_change: impl Fn(web_sys::Event) + 'static,
) -> View {
	let id = id.to_string();
	let name = name.to_string();
	let value = value.to_string();
	let label = label.to_string();

	page!(|
		id: String,
		name: String,
		value: String,
		label: String,
		checked: bool,
		on_change: impl Fn(web_sys::Event) + 'static
	| {
		div {
			class: "form-check",
			input {
				class: "form-check-input",
				type: "radio",
				id: id.clone(),
				name: name,
				value: value,
				checked: checked,
				onchange: on_change
			}
			label {
				class: "form-check-label",
				for: id,
				{label}
			}
		}
	})(id, name, value, label, checked, on_change)
}

pub fn submit_button(
	label: &str,
	loading: bool,
	loading_label: &str,
) -> View {
	let label = label.to_string();
	let loading_label = loading_label.to_string();

	page!(|label: String, loading: bool, loading_label: String| {
		button {
			class: "btn btn-primary",
			type: "submit",
			disabled: loading,
			if loading {
				{loading_label}
			} else {
				{label}
			}
		}
	})(label, loading, loading_label)
}
```

Usage in detail page:

```rust
use crate::client::components::forms::{radio_choice, submit_button};

pub fn polls_detail_page(question_id: i64) -> View {
	// ... state and event handlers

	page!(|choices_data: Vec<ChoiceInfo>, ...| {
		form {
			onsubmit: handle_submit,

			div {
				class: "mb-3",
				for choice in &choices_data {
					{radio_choice(
						&format!("choice{}", choice.id),
						"choice",
						&choice.id.to_string(),
						&choice.choice_text,
						selected == Some(choice.id),
						handle_choice_change.clone()
					)}
				}
			}

			{submit_button("Vote", voting_state, "Voting...")}
		}
	})(choices_data, ...)
}
```

### Pattern 3: Custom Hooks for Form State

For complex forms, create custom hooks:

```rust
// src/client/hooks/form.rs
use reinhardt_pages::prelude::*;

pub struct FormState<T> {
	pub value: T,
	pub error: Option<String>,
	pub loading: bool,
}

pub fn use_form_field<T: Clone + 'static>(
	initial: T
) -> (FormState<T>, impl Fn(T), impl Fn(Option<String>), impl Fn(bool)) {
	let (value, set_value) = use_state(initial);
	let (error, set_error) = use_state(None::<String>);
	let (loading, set_loading) = use_state(false);

	let state = FormState {
		value: value.get().clone(),
		error: error.get().clone(),
		loading: loading.get(),
	};

	(state, set_value, set_error, set_loading)
}
```

Usage:

```rust
use crate::client::hooks::form::use_form_field;

pub fn polls_detail_page(question_id: i64) -> View {
	let (choice_state, set_choice, set_choice_error, _) = use_form_field(None::<i64>);

	let handle_choice_change = {
		let set_choice = set_choice.clone();
		let set_choice_error = set_choice_error.clone();

		move |e: web_sys::Event| {
			if let Some(target) = e.target() {
				if let Some(input) = target.dyn_ref::<web_sys::HtmlInputElement>() {
					if let Ok(choice_id) = input.value().parse::<i64>() {
						set_choice(Some(choice_id));
						set_choice_error(None);
					}
				}
			}
		}
	};

	// ... rest of component
}
```

## Best Practices for Form Components

### 1. Controlled Components

Always use controlled components (state-driven):

```rust
// ✅ GOOD: Controlled component
input {
	value: email_state.clone(),
	oninput: handle_email_change
}

// ❌ BAD: Uncontrolled component
input {
	placeholder: "Email"
	// No value binding - state and UI can diverge
}
```

### 2. Immediate Validation Feedback

Clear errors when user starts typing:

```rust
let handle_email_change = {
	let set_email = set_email.clone();
	let set_email_error = set_email_error.clone();

	move |e: web_sys::Event| {
		if let Some(target) = e.target() {
			if let Some(input) = target.dyn_ref::<web_sys::HtmlInputElement>() {
				set_email(input.value());
				set_email_error(None);  // Clear error on change
			}
		}
	}
};
```

### 3. Disable Buttons During Submission

Prevent double submissions:

```rust
button {
	type: "submit",
	disabled: submitting_state || !is_valid_state,
	class: "btn btn-primary",
	if submitting_state {
		"Submitting..."
	} else {
		"Submit"
	}
}
```

### 4. Progressive Enhancement

Show loading states and optimistic updates:

```rust
spawn_local(async move {
	set_submitting(true);

	// Optimistic update (optional)
	update_ui_optimistically();

	match submit_form(data).await {
		Ok(result) => {
			// Success
			navigate_to_success_page();
		}
		Err(e) => {
			// Rollback optimistic update
			rollback_ui();
			set_error(Some(e.to_string()));
			set_submitting(false);
		}
	}
});
```

## Summary

In this tutorial, you learned:

- **Client-Side Form State**: Using `use_state()` for form data management
- **Event Handlers**: Attaching listeners to form elements
- **Client-Side Validation**: Immediate feedback before server submission
- **Server-Side Validation**: Security and data integrity at the server
- **CSRF Protection**: Current manual approach and future automatic integration
- **Component Patterns**: Reusable components instead of generic views
- **Custom Hooks**: Encapsulating form logic for reuse
- **Best Practices**: Controlled components, validation feedback, and progressive enhancement

**Key Differences from Traditional Approaches:**

| Aspect | Traditional (Tera) | reinhardt-pages |
|--------|-------------------|-----------------|
| Form Rendering | Server-side template | Client-side component |
| State Management | Server session | Client state (`use_state`) |
| Validation | Server-side only | Client + Server |
| CSRF Protection | Template tags (`{% csrf_token %}`) | Middleware integration (future) |
| Reusability | Generic views | Component composition |
| User Experience | Full page reload | Dynamic updates, no reload |

## What's Next?

In the next tutorial, we'll write automated tests for our reinhardt-pages application, including:

- Component testing
- Server function testing
- Integration testing with browser automation

Continue to [Part 5: Testing](5-testing.md).
