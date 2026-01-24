//! CSV export implementation

use super::ExportError;
use std::io::Write;

/// Exports data to CSV format
pub fn export_csv<W: Write>(_writer: &mut W, _data: &[Vec<String>]) -> Result<(), ExportError> {
	// TODO: Implement CSV export using csv crate
	todo!("CSV export will be implemented")
}
