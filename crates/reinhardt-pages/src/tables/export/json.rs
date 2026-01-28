//! JSON export implementation

use super::ExportError;
use std::io::Write;

/// Exports data to JSON format
pub fn export_json<W: Write>(_writer: &mut W, _data: &[Vec<String>]) -> Result<(), ExportError> {
	// TODO: Implement JSON export using serde_json
	todo!("JSON export will be implemented")
}
