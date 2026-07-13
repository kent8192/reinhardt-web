//! Type-erased descriptors for controlled form elements.

use std::fmt;
use std::num::IntErrorKind;
use std::sync::Arc;

use crate::reactive::Signal;

/// Identifies the form control represented by a binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlKind {
	/// A single-line or multi-line text control.
	Text,
	/// A numeric input control.
	Number,
	/// A checkbox control.
	Checkbox,
	/// A radio control.
	Radio,
	/// A single-selection control.
	SelectOne,
	/// A multiple-selection control.
	SelectMany,
}

impl fmt::Display for ControlKind {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Text => f.write_str("text"),
			Self::Number => f.write_str("number"),
			Self::Checkbox => f.write_str("checkbox"),
			Self::Radio => f.write_str("radio"),
			Self::SelectOne => f.write_str("select-one"),
			Self::SelectMany => f.write_str("select-many"),
		}
	}
}

/// Cross-target value read from or written to a form control.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlValue {
	/// A textual control value.
	Text(String),
	/// A checked-state control value.
	Checked(bool),
	/// Values selected by a multiple-selection control.
	SelectedValues(Vec<String>),
}

impl ControlValue {
	fn kind_name(&self) -> &'static str {
		match self {
			Self::Text(_) => "text",
			Self::Checked(_) => "checked",
			Self::SelectedValues(_) => "selected-values",
		}
	}
}

/// Stable classification for numeric input parsing failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumberParseErrorKind {
	/// The input is empty.
	Empty,
	/// The input is a valid prefix but not a complete number.
	Incomplete,
	/// The input is not a valid numeric lexeme.
	Invalid,
	/// The input cannot be represented by the target primitive.
	OutOfRange,
}

/// A numeric input parsing failure that retains the submitted text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumberParseError {
	raw: String,
	kind: NumberParseErrorKind,
}

impl NumberParseError {
	fn new(raw: &str, kind: NumberParseErrorKind) -> Self {
		Self {
			raw: raw.to_owned(),
			kind,
		}
	}

	/// Returns the unmodified input text.
	pub fn raw(&self) -> &str {
		&self.raw
	}

	/// Returns the stable failure classification.
	pub fn kind(&self) -> NumberParseErrorKind {
		self.kind
	}
}

impl fmt::Display for NumberParseError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(
			f,
			"cannot parse numeric control value {:?}: {:?}",
			self.raw, self.kind
		)
	}
}

impl std::error::Error for NumberParseError {}

/// Result of applying a control value to its bound signal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlWriteOutcome {
	/// The signal was updated.
	Committed,
	/// A numeric value was rejected without changing the numeric signal.
	Rejected(NumberParseError),
	/// The input did not require a signal update.
	Ignored,
}

/// Framework-level failure while reading or writing a bound form control.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ControlBindingError {
	/// The normalized value does not match the binding's control kind.
	ValueKindMismatch {
		/// The binding's control kind.
		control: ControlKind,
		/// The normalized value kind that was supplied.
		actual: &'static str,
	},
	/// The binding cannot be attached to the supplied element.
	UnsupportedElement {
		/// The binding's control kind.
		control: ControlKind,
		/// The element tag that was supplied.
		actual_tag: String,
	},
	/// A required DOM property is unavailable.
	MissingProperty {
		/// The binding's control kind.
		control: ControlKind,
		/// The missing property name.
		property: &'static str,
	},
}

impl fmt::Display for ControlBindingError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::ValueKindMismatch { control, actual } => {
				write!(f, "{control} control cannot accept a {actual} value")
			}
			Self::UnsupportedElement {
				control,
				actual_tag,
			} => write!(
				f,
				"{control} control does not support a <{actual_tag}> element"
			),
			Self::MissingProperty { control, property } => {
				write!(f, "{control} control is missing the {property} property")
			}
		}
	}
}

impl std::error::Error for ControlBindingError {}

type ReadValue = Arc<dyn Fn() -> ControlValue + 'static>;
type WriteValue =
	Arc<dyn Fn(ControlValue) -> Result<ControlWriteOutcome, ControlBindingError> + 'static>;

/// Cloneable type-erased reader and writer for a controlled form element.
#[derive(Clone)]
pub struct ControlBinding {
	kind: ControlKind,
	radio_value: Option<String>,
	read: ReadValue,
	write: WriteValue,
}

impl fmt::Debug for ControlBinding {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("ControlBinding")
			.field("kind", &self.kind)
			.field("radio_value", &self.radio_value)
			.finish_non_exhaustive()
	}
}

impl ControlBinding {
	/// Creates a binding for a textual signal.
	pub fn text(signal: Signal<String>) -> Self {
		Self::string_value(ControlKind::Text, signal)
	}

	/// Creates a binding for a checkbox signal.
	pub fn checkbox(signal: Signal<bool>) -> Self {
		let read_signal = signal.clone();
		Self {
			kind: ControlKind::Checkbox,
			radio_value: None,
			read: Arc::new(move || ControlValue::Checked(read_signal.get())),
			write: Arc::new(move |value| match value {
				ControlValue::Checked(value) => {
					signal.set(value);
					Ok(ControlWriteOutcome::Committed)
				}
				actual => Err(value_kind_mismatch(ControlKind::Checkbox, &actual)),
			}),
		}
	}

	/// Creates a binding for one radio choice within a string-valued group.
	pub fn radio(signal: Signal<String>, value: String) -> Self {
		let read_signal = signal.clone();
		let read_value = value.clone();
		let write_value = value.clone();
		Self {
			kind: ControlKind::Radio,
			radio_value: Some(value),
			read: Arc::new(move || ControlValue::Checked(read_signal.get() == read_value)),
			write: Arc::new(move |value| match value {
				ControlValue::Checked(true) => {
					signal.set(write_value.clone());
					Ok(ControlWriteOutcome::Committed)
				}
				ControlValue::Checked(false) => Ok(ControlWriteOutcome::Ignored),
				actual => Err(value_kind_mismatch(ControlKind::Radio, &actual)),
			}),
		}
	}

	/// Creates a binding for a single-selection signal.
	pub fn select_one(signal: Signal<String>) -> Self {
		Self::string_value(ControlKind::SelectOne, signal)
	}

	/// Creates a binding for a multiple-selection signal.
	pub fn select_many(signal: Signal<Vec<String>>) -> Self {
		let read_signal = signal.clone();
		Self {
			kind: ControlKind::SelectMany,
			radio_value: None,
			read: Arc::new(move || ControlValue::SelectedValues(read_signal.get())),
			write: Arc::new(move |value| match value {
				ControlValue::SelectedValues(values) => {
					signal.set(values);
					Ok(ControlWriteOutcome::Committed)
				}
				actual => Err(value_kind_mismatch(ControlKind::SelectMany, &actual)),
			}),
		}
	}

	/// Creates a numeric binding without an application-visible error signal.
	pub fn number<T: NumberValue>(signal: Signal<T>) -> Self {
		Self::number_binding(signal, None)
	}

	/// Creates a numeric binding that reports rejected input through a signal.
	pub fn number_with_error<T: NumberValue>(
		signal: Signal<T>,
		error: Signal<Option<NumberParseError>>,
	) -> Self {
		Self::number_binding(signal, Some(error))
	}

	/// Returns the binding's control kind.
	pub fn kind(&self) -> ControlKind {
		self.kind
	}

	/// Returns the configured radio choice, if this is a radio binding.
	pub fn radio_value(&self) -> Option<&str> {
		self.radio_value.as_deref()
	}

	/// Reads the current signal value in its cross-target representation.
	pub fn read(&self) -> ControlValue {
		(self.read)()
	}

	/// Applies a cross-target value to the bound signal.
	pub fn write(&self, value: ControlValue) -> Result<ControlWriteOutcome, ControlBindingError> {
		(self.write)(value)
	}

	fn string_value(kind: ControlKind, signal: Signal<String>) -> Self {
		let read_signal = signal.clone();
		Self {
			kind,
			radio_value: None,
			read: Arc::new(move || ControlValue::Text(read_signal.get())),
			write: Arc::new(move |value| match value {
				ControlValue::Text(value) => {
					signal.set(value);
					Ok(ControlWriteOutcome::Committed)
				}
				actual => Err(value_kind_mismatch(kind, &actual)),
			}),
		}
	}

	fn number_binding<T: NumberValue>(
		signal: Signal<T>,
		error: Option<Signal<Option<NumberParseError>>>,
	) -> Self {
		let read_signal = signal.clone();
		Self {
			kind: ControlKind::Number,
			radio_value: None,
			read: Arc::new(move || ControlValue::Text(read_signal.get().to_string())),
			write: Arc::new(move |value| {
				let ControlValue::Text(raw) = value else {
					return Err(value_kind_mismatch(ControlKind::Number, &value));
				};

				match T::parse_control_value(&raw) {
					Ok(value) => {
						signal.set(value);
						if let Some(error) = &error {
							error.set(None);
						}
						Ok(ControlWriteOutcome::Committed)
					}
					Err(parse_error) => {
						if let Some(error) = &error {
							error.set(Some(parse_error.clone()));
						}
						Ok(ControlWriteOutcome::Rejected(parse_error))
					}
				}
			}),
		}
	}
}

fn value_kind_mismatch(control: ControlKind, actual: &ControlValue) -> ControlBindingError {
	ControlBindingError::ValueKindMismatch {
		control,
		actual: actual.kind_name(),
	}
}

mod sealed {
	pub trait Sealed {}
}

/// Primitive numeric type supported by controlled numeric inputs.
pub trait NumberValue: sealed::Sealed + Clone + fmt::Display + 'static {
	/// Parses a complete control value into this primitive.
	fn parse_control_value(raw: &str) -> Result<Self, NumberParseError>;
}

fn lexical_error(raw: &str) -> Option<NumberParseError> {
	if raw.is_empty() {
		return Some(NumberParseError::new(raw, NumberParseErrorKind::Empty));
	}
	if is_incomplete_number(raw) {
		return Some(NumberParseError::new(raw, NumberParseErrorKind::Incomplete));
	}
	None
}

fn is_incomplete_number(raw: &str) -> bool {
	if matches!(raw, "+" | "-") {
		return true;
	}

	let unsigned = raw.strip_prefix(['+', '-']).unwrap_or(raw);
	if let Some(integer) = unsigned.strip_suffix('.')
		&& !integer.is_empty()
		&& integer.bytes().all(|byte| byte.is_ascii_digit())
	{
		return true;
	}

	let Some(exponent_index) = raw.rfind(['e', 'E']) else {
		return false;
	};
	let (significand, exponent) = raw.split_at(exponent_index);
	let exponent = &exponent[1..];
	!significand.is_empty()
		&& matches!(exponent, "" | "+" | "-")
		&& significand.parse::<f64>().is_ok()
}

macro_rules! impl_signed_number_value {
	($($type:ty),+ $(,)?) => {
		$(
			impl sealed::Sealed for $type {}

			impl NumberValue for $type {
				fn parse_control_value(raw: &str) -> Result<Self, NumberParseError> {
					if let Some(error) = lexical_error(raw) {
						return Err(error);
					}
					raw.parse::<Self>().map_err(|error| {
						let kind = match error.kind() {
							IntErrorKind::PosOverflow | IntErrorKind::NegOverflow => {
								NumberParseErrorKind::OutOfRange
							}
							_ => NumberParseErrorKind::Invalid,
						};
						NumberParseError::new(raw, kind)
					})
				}
			}
		)+
	};
}

macro_rules! impl_unsigned_number_value {
	($($type:ty),+ $(,)?) => {
		$(
			impl sealed::Sealed for $type {}

			impl NumberValue for $type {
				fn parse_control_value(raw: &str) -> Result<Self, NumberParseError> {
					if let Some(error) = lexical_error(raw) {
						return Err(error);
					}
					if raw
						.strip_prefix('-')
						.is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit()))
					{
						return Err(NumberParseError::new(raw, NumberParseErrorKind::OutOfRange));
					}
					raw.parse::<Self>().map_err(|error| {
						let kind = match error.kind() {
							IntErrorKind::PosOverflow | IntErrorKind::NegOverflow => {
								NumberParseErrorKind::OutOfRange
							}
							_ => NumberParseErrorKind::Invalid,
						};
						NumberParseError::new(raw, kind)
					})
				}
			}
		)+
	};
}

macro_rules! impl_float_number_value {
	($($type:ty),+ $(,)?) => {
		$(
			impl sealed::Sealed for $type {}

			impl NumberValue for $type {
				fn parse_control_value(raw: &str) -> Result<Self, NumberParseError> {
					if let Some(error) = lexical_error(raw) {
						return Err(error);
					}
					let value = raw
						.parse::<Self>()
						.map_err(|_| NumberParseError::new(raw, NumberParseErrorKind::Invalid))?;
					if !value.is_finite() {
						return Err(NumberParseError::new(raw, NumberParseErrorKind::OutOfRange));
					}
					Ok(value)
				}
			}
		)+
	};
}

impl_signed_number_value!(i8, i16, i32, i64, i128, isize);
impl_unsigned_number_value!(u8, u16, u32, u64, u128, usize);
impl_float_number_value!(f32, f64);

#[cfg(test)]
mod tests {
	use super::*;
	use crate::reactive::Signal;
	use rstest::rstest;

	#[rstest]
	#[case("", NumberParseErrorKind::Empty)]
	#[case("-", NumberParseErrorKind::Incomplete)]
	#[case("1.", NumberParseErrorKind::Incomplete)]
	#[case("1e-", NumberParseErrorKind::Incomplete)]
	#[case("abc", NumberParseErrorKind::Invalid)]
	fn number_binding_preserves_last_value_on_invalid_input(
		#[case] raw: &str,
		#[case] kind: NumberParseErrorKind,
	) {
		// Arrange
		let value = Signal::new(7.5_f64);
		let error = Signal::new(None);
		let binding = ControlBinding::number_with_error(value.clone(), error.clone());

		// Act
		let outcome = binding.write(ControlValue::Text(raw.to_owned())).unwrap();

		// Assert
		assert_eq!(value.get(), 7.5);
		assert_eq!(outcome, ControlWriteOutcome::Rejected(error.get().unwrap()));
		assert_eq!(error.get().unwrap().kind(), kind);
	}

	#[rstest]
	fn text_binding_reads_and_writes_the_signal() {
		// Arrange
		let signal = Signal::new("old".to_owned());
		let binding = ControlBinding::text(signal.clone());

		// Act
		binding.write(ControlValue::Text("new".to_owned())).unwrap();

		// Assert
		assert_eq!(binding.read(), ControlValue::Text("new".to_owned()));
		assert_eq!(signal.get(), "new");
	}
}
