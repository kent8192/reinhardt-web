//! Stable ordering and accessible relationships for retained validation regions.

use crate::FieldError;
use std::{collections::HashMap, hash::Hash};

/// One summary message and, for field errors, the retained control it describes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SummaryEntry {
	/// Associated control ID for a field error.
	pub control_id: Option<String>,
	/// Existing runtime message, without rewriting or parsing it.
	pub message: String,
}

/// Orders field messages by declaration and deduplicates equal global messages.
pub fn collect_summary<Field: Copy + Eq + Hash>(
	ordered_fields: &[(Field, String)],
	errors: &HashMap<Field, FieldError>,
	form_error: Option<&str>,
	submit_error: Option<&str>,
) -> Vec<SummaryEntry> {
	let mut entries: Vec<_> = ordered_fields
		.iter()
		.filter_map(|(field, id)| {
			errors.get(field).map(|error| SummaryEntry {
				control_id: Some(id.clone()),
				message: error.message().into(),
			})
		})
		.collect();
	for message in [form_error, submit_error]
		.into_iter()
		.flatten()
		.filter(|message| !message.is_empty())
	{
		if !entries
			.iter()
			.any(|entry| entry.control_id.is_none() && entry.message == message)
		{
			entries.push(SummaryEntry {
				control_id: None,
				message: message.into(),
			});
		}
	}
	entries
}

/// Combines generated and external IDs, retaining each ID's first occurrence.
pub fn describedby(help_id: Option<&str>, error_id: &str, external: Option<&str>) -> String {
	let mut ids = Vec::new();
	for id in help_id
		.into_iter()
		.chain(Some(error_id))
		.chain(external.into_iter().flat_map(str::split_ascii_whitespace))
	{
		if !ids.contains(&id) {
			ids.push(id);
		}
	}
	ids.join(" ")
}
