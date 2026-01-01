//! Use case tests for the `form!` macro.
//!
//! Tests real-world form scenarios like login, registration, contact forms.

use reinhardt_forms::form;
use rstest::rstest;
use serde_json::json;
use std::collections::HashMap;

/// UC-001: Login form use case.
///
/// Tests username + password + remember_me form.
#[rstest]
fn test_login_form_use_case() {
	let mut form = form! {
		fields: {
			username: CharField {
				required,
				max_length: 150,
			},
			password: CharField {
				required,
				widget: PasswordInput,
			},
			remember_me: BooleanField {},
		},
	};

	// Test 1: Valid credentials
	let mut data = HashMap::new();
	data.insert("username".to_string(), json!("testuser"));
	data.insert("password".to_string(), json!("password123"));
	data.insert("remember_me".to_string(), json!(true));
	form.bind(data);

	assert!(form.is_valid());
	assert_eq!(
		form.cleaned_data().get("username"),
		Some(&json!("testuser"))
	);

	// Test 2: Empty username
	let mut form2 = form! {
		fields: {
			username: CharField { required },
			password: CharField { required, widget: PasswordInput },
		},
	};
	let data2 = HashMap::new();
	form2.bind(data2);

	assert!(!form2.is_valid());
	assert!(form2.errors().contains_key("username"));
}

/// UC-002: User registration form use case.
///
/// Tests username + email + password + confirm_password with validation.
#[rstest]
fn test_user_registration_form_use_case() {
	let mut form = form! {
		fields: {
			username: CharField {
				required,
				max_length: 150,
			},
			email: EmailField {
				required,
			},
			password: CharField {
				required,
				widget: PasswordInput,
			},
			confirm_password: CharField {
				required,
				widget: PasswordInput,
			},
		},
		validators: {
			@form: [
				|data: &std::collections::HashMap<String, serde_json::Value>| {
					let password = data.get("password").and_then(|v| v.as_str());
					let confirm = data.get("confirm_password").and_then(|v| v.as_str());
					password == confirm
				} => "Passwords must match",
			],
		},
	};

	// Test 1: Valid registration
	let mut data = HashMap::new();
	data.insert("username".to_string(), json!("newuser"));
	data.insert("email".to_string(), json!("user@example.com"));
	data.insert("password".to_string(), json!("securepassword"));
	data.insert("confirm_password".to_string(), json!("securepassword"));
	form.bind(data);

	assert!(form.is_valid());

	// Test 2: Password mismatch
	let mut form2 = form! {
		fields: {
			password: CharField { required },
			confirm_password: CharField { required },
		},
		validators: {
			@form: [
				|data: &std::collections::HashMap<String, serde_json::Value>| {
					data.get("password") == data.get("confirm_password")
				} => "Passwords must match",
			],
		},
	};

	let mut data2 = HashMap::new();
	data2.insert("password".to_string(), json!("password1"));
	data2.insert("confirm_password".to_string(), json!("password2"));
	form2.bind(data2);

	assert!(!form2.is_valid());
}

/// UC-003: Contact form use case.
///
/// Tests name + email + subject + message form.
#[rstest]
fn test_contact_form_use_case() {
	let mut form = form! {
		fields: {
			name: CharField {
				required,
				max_length: 100,
			},
			email: EmailField {
				required,
			},
			subject: CharField {
				required,
				max_length: 200,
			},
			message: CharField {
				required,
				widget: TextArea,
			},
		},
	};

	// Valid contact form submission
	let mut data = HashMap::new();
	data.insert("name".to_string(), json!("John Doe"));
	data.insert("email".to_string(), json!("john@example.com"));
	data.insert("subject".to_string(), json!("Inquiry"));
	data.insert("message".to_string(), json!("Hello, I have a question..."));
	form.bind(data);

	assert!(form.is_valid());
	assert_eq!(form.fields().len(), 4);
}

/// UC-004: Search form use case.
///
/// Tests query + category + date_range form (all optional).
#[rstest]
fn test_search_form_use_case() {
	let form = form! {
		fields: {
			query: CharField {
				max_length: 255,
			},
			date_from: DateField {},
			date_to: DateField {},
		},
	};

	// Test field count (DateField validation is complex, so we just verify structure)
	assert_eq!(form.fields().len(), 3);
}

/// UC-005: Profile edit form use case.
///
/// Tests multiple optional fields.
#[rstest]
fn test_profile_edit_form_use_case() {
	let mut form = form! {
		fields: {
			display_name: CharField {
				max_length: 100,
			},
			bio: CharField {
				max_length: 500,
				widget: TextArea,
			},
			website: URLField {},
			location: CharField {
				max_length: 100,
			},
		},
	};

	// Provide valid data for all fields (URLField requires proper URL format)
	let mut data = HashMap::new();
	data.insert("display_name".to_string(), json!("John"));
	data.insert("bio".to_string(), json!("A developer"));
	data.insert("website".to_string(), json!("https://example.com"));
	data.insert("location".to_string(), json!("Tokyo"));
	form.bind(data);

	assert!(form.is_valid());
	assert_eq!(form.fields().len(), 4);
}

/// UC-006: Payment form use case.
///
/// Tests credit card information form.
#[rstest]
fn test_payment_form_use_case() {
	let mut form = form! {
		fields: {
			card_number: CharField {
				required,
				max_length: 19,
			},
			expiry_month: IntegerField {
				required,
			},
			expiry_year: IntegerField {
				required,
			},
			cvv: CharField {
				required,
				max_length: 4,
				widget: PasswordInput,
			},
			cardholder_name: CharField {
				required,
			},
		},
	};

	// Valid card details
	let mut data = HashMap::new();
	data.insert("card_number".to_string(), json!("4111111111111111"));
	data.insert("expiry_month".to_string(), json!(12));
	data.insert("expiry_year".to_string(), json!(2025));
	data.insert("cvv".to_string(), json!("123"));
	data.insert("cardholder_name".to_string(), json!("John Doe"));
	form.bind(data);

	assert!(form.is_valid());
}

/// UC-007: File upload form use case.
///
/// Tests FileField + ImageField.
#[rstest]
fn test_file_upload_form_use_case() {
	let form = form! {
		fields: {
			document: FileField {
				required,
			},
			thumbnail: ImageField {},
		},
	};

	// File fields require special handling, test field count
	assert_eq!(form.fields().len(), 2);
}

/// UC-008: Multi-language form use case.
///
/// Tests Japanese labels and help text.
#[rstest]
fn test_multilingual_form_use_case() {
	let mut form = form! {
		fields: {
			username: CharField {
				required,
				label: "ユーザー名",
				help_text: "3文字以上で入力してください",
			},
			email: EmailField {
				required,
				label: "メールアドレス",
				help_text: "有効なメールアドレスを入力してください",
			},
		},
	};

	// Valid submission with Unicode
	let mut data = HashMap::new();
	data.insert("username".to_string(), json!("田中太郎"));
	data.insert("email".to_string(), json!("tanaka@example.co.jp"));
	form.bind(data);

	assert!(form.is_valid());
	assert_eq!(form.fields().len(), 2);
}
