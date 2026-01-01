//! HP-010: Complex form with all features.
//!
//! Tests that a complex form combining all features compiles successfully.

use reinhardt_forms_macros::form;

fn main() {
	let _form = form! {
		name: "registration_form",
		fields: {
			username: CharField {
				required,
				max_length: 150,
				min_length: 3,
				label: "Username",
				help_text: "Choose a unique username",
			},
			email: EmailField {
				required,
				label: "Email Address",
			},
			password: CharField {
				required,
				min_length: 8,
				widget: PasswordInput,
				label: "Password",
				help_text: "Must be at least 8 characters",
			},
			confirm_password: CharField {
				required,
				widget: PasswordInput,
				label: "Confirm Password",
			},
			age: IntegerField {
				min_value: 18,
				max_value: 120,
				label: "Age",
			},
			bio: CharField {
				max_length: 500,
				widget: TextArea,
				label: "Biography",
				help_text: "Tell us about yourself",
			},
			website: URLField {
				label: "Website",
			},
			avatar: ImageField {
				label: "Profile Picture",
			},
			terms_accepted: BooleanField {
				required,
				label: "I accept the terms and conditions",
			},
		},
		validators: {
			username: [
				|v| !v.contains(' ') => "Username cannot contain spaces",
				|v| v.chars().all(|c| c.is_alphanumeric() || c == '_')
					=> "Username can only contain letters, numbers, and underscores",
			],
			password: [
				|v| v.chars().any(|c| c.is_uppercase())
					=> "Password must contain an uppercase letter",
				|v| v.chars().any(|c| c.is_numeric())
					=> "Password must contain a number",
			],
			@form: [
				|data| data["password"] == data["confirm_password"]
					=> "Passwords must match",
			],
		},
		client_validators: {
			username: [
				"value.length >= 3" => "Username must be at least 3 characters",
				"!/\\s/.test(value)" => "Username cannot contain spaces",
			],
			password: [
				"value.length >= 8" => "Password must be at least 8 characters",
				"/[A-Z]/.test(value)" => "Password must contain an uppercase letter",
				"/[0-9]/.test(value)" => "Password must contain a number",
			],
		},
	};
}
