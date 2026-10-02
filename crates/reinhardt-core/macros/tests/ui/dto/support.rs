pub mod validators {
	use std::borrow::Cow;
	use std::collections::BTreeMap;

	#[derive(Debug, Clone)]
	pub enum ValidationError {
		Custom(String),
	}

	#[derive(Debug, Clone)]
	pub struct ValidationErrors {
		errors: BTreeMap<Cow<'static, str>, Vec<ValidationError>>,
	}

	impl ValidationErrors {
		pub fn new() -> Self {
			Self {
				errors: BTreeMap::new(),
			}
		}

		pub fn add(&mut self, field: impl Into<Cow<'static, str>>, error: ValidationError) {
			self.errors.entry(field.into()).or_default().push(error);
		}

		pub fn is_empty(&self) -> bool {
			self.errors.is_empty()
		}
	}

	pub trait Validate {
		fn validate(&self) -> Result<(), ValidationErrors>;
	}

	pub trait Validator<T: ?Sized> {
		fn validate(&self, value: &T) -> Result<(), ValidationError>;
	}

	pub struct EmailValidator;

	impl EmailValidator {
		pub fn new() -> Self {
			Self
		}
	}

	impl Validator<str> for EmailValidator {
		fn validate(&self, _value: &str) -> Result<(), ValidationError> {
			Ok(())
		}
	}

	impl Validator<String> for EmailValidator {
		fn validate(&self, value: &String) -> Result<(), ValidationError> {
			<Self as Validator<str>>::validate(self, value)
		}
	}

	pub struct UrlValidator;

	impl UrlValidator {
		pub fn new() -> Self {
			Self
		}
	}

	impl Validator<str> for UrlValidator {
		fn validate(&self, _value: &str) -> Result<(), ValidationError> {
			Ok(())
		}
	}

	impl Validator<String> for UrlValidator {
		fn validate(&self, value: &String) -> Result<(), ValidationError> {
			<Self as Validator<str>>::validate(self, value)
		}
	}

	pub struct MinLengthValidator {
		min: usize,
	}

	impl MinLengthValidator {
		pub fn new(min: usize) -> Self {
			Self { min }
		}
	}

	impl Validator<str> for MinLengthValidator {
		fn validate(&self, value: &str) -> Result<(), ValidationError> {
			if value.chars().count() >= self.min {
				Ok(())
			} else {
				Err(ValidationError::Custom("too short".to_string()))
			}
		}
	}

	impl Validator<String> for MinLengthValidator {
		fn validate(&self, value: &String) -> Result<(), ValidationError> {
			<Self as Validator<str>>::validate(self, value)
		}
	}

	pub struct MaxLengthValidator {
		max: usize,
	}

	impl MaxLengthValidator {
		pub fn new(max: usize) -> Self {
			Self { max }
		}
	}

	impl Validator<str> for MaxLengthValidator {
		fn validate(&self, value: &str) -> Result<(), ValidationError> {
			if value.chars().count() <= self.max {
				Ok(())
			} else {
				Err(ValidationError::Custom("too long".to_string()))
			}
		}
	}

	impl Validator<String> for MaxLengthValidator {
		fn validate(&self, value: &String) -> Result<(), ValidationError> {
			<Self as Validator<str>>::validate(self, value)
		}
	}

	pub struct MinValueValidator<T> {
		min: T,
	}

	impl<T> MinValueValidator<T> {
		pub fn new(min: T) -> Self {
			Self { min }
		}
	}

	impl<T: PartialOrd> Validator<T> for MinValueValidator<T> {
		fn validate(&self, value: &T) -> Result<(), ValidationError> {
			if value >= &self.min {
				Ok(())
			} else {
				Err(ValidationError::Custom("too small".to_string()))
			}
		}
	}

	pub struct MaxValueValidator<T> {
		max: T,
	}

	impl<T> MaxValueValidator<T> {
		pub fn new(max: T) -> Self {
			Self { max }
		}
	}

	impl<T: PartialOrd> Validator<T> for MaxValueValidator<T> {
		fn validate(&self, value: &T) -> Result<(), ValidationError> {
			if value <= &self.max {
				Ok(())
			} else {
				Err(ValidationError::Custom("too large".to_string()))
			}
		}
	}
}
