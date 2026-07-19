//! WASM-side diagnostic generation and stale-update filtering.

use super::protocol::{BuildDiagnostic, PatchGeneration};

/// Retains the newest compiler diagnostics shown by the HMR overlay.
#[cfg(wasm)]
#[derive(Default)]
pub struct DiagnosticStore {
	generation: Option<PatchGeneration>,
	diagnostics: Vec<BuildDiagnostic>,
}

#[cfg(wasm)]
impl DiagnosticStore {
	/// Creates an empty diagnostic store.
	pub fn new() -> Self {
		Self::default()
	}

	/// Replaces diagnostics when the update is not stale.
	pub fn apply(
		&mut self,
		generation: PatchGeneration,
		diagnostics: Vec<BuildDiagnostic>,
	) -> bool {
		if self.generation.is_some_and(|current| generation < current) {
			return false;
		}
		self.generation = Some(generation);
		self.diagnostics = diagnostics;
		true
	}

	/// Clears diagnostics after a newer successful generation.
	pub fn clear(&mut self, generation: PatchGeneration) -> bool {
		if self.generation.is_some_and(|current| generation < current) {
			return false;
		}
		self.generation = Some(generation);
		self.diagnostics.clear();
		true
	}

	/// Returns the newest accepted generation.
	pub fn generation(&self) -> Option<PatchGeneration> {
		self.generation
	}

	/// Returns the currently displayed diagnostics.
	pub fn diagnostics(&self) -> &[BuildDiagnostic] {
		&self.diagnostics
	}
}
