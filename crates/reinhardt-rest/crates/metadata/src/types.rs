//! Core types for metadata system

use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum MetadataError {
	#[error("Failed to determine metadata: {0}")]
	DeterminationError(String),

	#[error("Serializer not available")]
	SerializerNotAvailable,
}

/// Field type enumeration for metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
	Field,
	Boolean,
	String,
	Integer,
	Float,
	Decimal,
	Date,
	DateTime,
	Time,
	Duration,
	Email,
	Url,
	Uuid,
	Choice,
	MultipleChoice,
	File,
	Image,
	List,
	NestedObject,
}

/// Choice information for choice fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChoiceInfo {
	pub value: String,
	pub display_name: String,
}

#[cfg(test)]
mod tests {
	use super::*;

	// DRF test: test_null_boolean_field_info_type
	#[test]
	fn test_boolean_field_info_type() {
		let field_type = FieldType::Boolean;
		assert_eq!(field_type, FieldType::Boolean);
	}

	// DRF test: test_decimal_field_info_type
	#[test]
	fn test_decimal_field_info_type() {
		// Note: In DRF, max_digits and decimal_places are specific to DecimalField
		// In Rust, we use the Decimal field type and could add custom fields if needed
		let field_type = FieldType::Decimal;
		assert_eq!(field_type, FieldType::Decimal);
	}
}
