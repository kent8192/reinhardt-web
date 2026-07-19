//! Compiler-side coordination for template patches and fallback rebuilds.

use std::path::PathBuf;

use reinhardt_pages::hmr::{
	BuildDiagnostic, CompiledBuildId, DiagnosticLevel, DiagnosticSpan, DiagnosticTarget, HmrServer,
	PatchGeneration, PatchRejection, TemplatePatch,
};

use crate::{CompiledBaseline, StaticOverlayStore, TemplateClassification, classify_source_change};

/// A successful client build and its template manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateBuildArtifact {
	/// Identity of the generated WASM build.
	pub build_id: CompiledBuildId,
	/// Digest of the generated template manifest.
	pub manifest_digest: [u8; 32],
	/// Source baseline captured by the successful build.
	pub baseline: CompiledBaseline,
}

/// Errors while installing a successful template build.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CoordinatorError {
	/// The artifact identity did not match the embedded baseline identity.
	#[error("baseline identity does not match the successful build")]
	BaselineInstall(String),
	/// A build result arrived after a newer generation was already installed.
	#[error("stale template generation: {0:?}")]
	StaleGeneration(PatchGeneration),
}

/// Result of dispatching a source change or client event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchOutcome {
	/// A compatible patch batch was sent to the browser.
	PatchSent(PatchGeneration),
	/// The normal WASM/server rebuild path should run.
	RebuildStarted(PatchGeneration),
	/// A template diagnostic was published.
	DiagnosticPublished(PatchGeneration),
	/// The event was older than the active generation.
	IgnoredStale(PatchGeneration),
}

#[derive(Debug, Clone)]
struct PendingGeneration {
	generation: PatchGeneration,
	paths: Vec<PathBuf>,
	patches: Vec<TemplatePatch>,
	fallback_started: bool,
}

/// Coordinates source classification, overlay state, and fallback rebuilds.
pub struct TemplateHotReloadCoordinator {
	project_root: PathBuf,
	baseline: Option<CompiledBaseline>,
	overlays: StaticOverlayStore,
	hmr_server: Option<HmrServer>,
	next_generation: PatchGeneration,
	pending: Option<PendingGeneration>,
}

impl TemplateHotReloadCoordinator {
	/// Creates a coordinator for one project and browser HMR channel.
	pub fn new(project_root: impl Into<PathBuf>, hmr_server: Option<HmrServer>) -> Self {
		Self {
			project_root: project_root.into(),
			baseline: None,
			overlays: StaticOverlayStore::new(),
			hmr_server,
			next_generation: PatchGeneration(0),
			pending: None,
		}
	}

	/// Classifies changed sources and dispatches a patch or fallback build signal.
	pub fn classify_and_dispatch(&mut self, paths: Vec<PathBuf>) -> DispatchOutcome {
		let generation = self.allocate_generation();
		let Some(baseline) = &self.baseline else {
			return self.start_rebuild(generation, paths);
		};
		match classify_source_change(&self.project_root, &paths, baseline, &self.overlays) {
			TemplateClassification::Patchable(patch_set) => {
				if patch_set.patches.is_empty() {
					return DispatchOutcome::IgnoredStale(generation);
				}
				let patches = patch_set.patches;
				self.pending = Some(PendingGeneration {
					generation,
					paths,
					patches: patches.clone(),
					fallback_started: false,
				});
				self.notify_template_patch(reinhardt_pages::hmr::TemplatePatchBatch {
					build_id: patch_set.build_id,
					manifest_digest: patch_set.manifest_digest,
					generation,
					patches,
				});
				DispatchOutcome::PatchSent(generation)
			}
			TemplateClassification::RebuildRequired(_) => self.start_rebuild(generation, paths),
			TemplateClassification::InvalidTemplate(diagnostic) => {
				self.notify_diagnostics(vec![diagnostic_to_build_diagnostic(
					generation, diagnostic,
				)]);
				self.start_rebuild(generation, paths)
			}
		}
	}

	/// Handles a browser patch rejection with at most one fallback per generation.
	pub fn handle_patch_rejection(
		&mut self,
		generation: PatchGeneration,
		rejection: PatchRejection,
	) -> DispatchOutcome {
		let generation = match self.pending.as_mut() {
			Some(pending)
				if pending.generation == generation
					&& !pending.patches.is_empty()
					&& !pending.fallback_started =>
			{
				pending.fallback_started = true;
				pending.generation
			}
			Some(pending) => return DispatchOutcome::IgnoredStale(pending.generation),
			None => return DispatchOutcome::IgnoredStale(self.overlays.generation),
		};
		self.notify_build_started(generation);
		let _ = rejection;
		DispatchOutcome::RebuildStarted(generation)
	}

	/// Installs the source manifest captured by the initial successful client
	/// build without emitting a browser lifecycle message.
	pub fn install_initial_baseline(&mut self, baseline: CompiledBaseline) {
		self.baseline = Some(baseline);
		self.overlays.clear(self.next_generation);
		self.pending = None;
	}

	/// Replaces the successful source manifest after a normal full rebuild.
	///
	/// The caller has already sent the normal reload notification, so this
	/// deliberately does not broadcast another lifecycle event.
	pub fn refresh_successful_baseline(&mut self, baseline: CompiledBaseline) {
		self.baseline = Some(baseline);
		self.overlays.clear(self.next_generation);
		self.pending = None;
	}

	/// Installs a successful build and clears overlays only after identity validation.
	pub fn install_successful_build(
		&mut self,
		artifact: TemplateBuildArtifact,
	) -> Result<(), CoordinatorError> {
		if artifact.baseline.build_id != artifact.build_id
			|| artifact.baseline.manifest_digest != artifact.manifest_digest
		{
			return Err(CoordinatorError::BaselineInstall(
				"artifact and baseline identities differ".to_owned(),
			));
		}
		let generation = match self.pending.as_ref() {
			Some(pending) => pending.generation,
			None => self.allocate_generation(),
		};
		if generation < self.overlays.generation {
			return Err(CoordinatorError::StaleGeneration(generation));
		}
		self.baseline = Some(artifact.baseline);
		self.overlays.clear(generation);
		self.pending = None;
		self.notify_build_recovered(generation);
		self.notify_full_reload("WASM build completed successfully");
		Ok(())
	}

	/// Commits overlays after the browser acknowledges a patch generation.
	pub fn handle_patch_applied(&mut self, generation: PatchGeneration) -> DispatchOutcome {
		let Some(pending) = self.pending.take() else {
			return DispatchOutcome::IgnoredStale(generation);
		};
		if pending.generation != generation || pending.fallback_started {
			self.pending = Some(pending);
			return DispatchOutcome::IgnoredStale(generation);
		}
		self.overlays.install(generation, &pending.patches);
		DispatchOutcome::PatchSent(generation)
	}

	/// Returns the changed paths whose pending patch rejection requires a
	/// fallback build. The watcher owns the actual pipeline invocation.
	pub fn pending_rebuild_paths(&self) -> Option<&[PathBuf]> {
		self.pending
			.as_ref()
			.filter(|pending| pending.patches.is_empty() || pending.fallback_started)
			.map(|pending| pending.paths.as_slice())
	}

	/// Publishes a build failure while retaining the last known-good baseline
	/// and browser process for a subsequent fix-and-save retry.
	pub fn publish_build_failure(
		&self,
		generation: PatchGeneration,
		target: DiagnosticTarget,
		message: impl Into<String>,
	) {
		let message = message.into();
		self.notify_diagnostics(vec![BuildDiagnostic {
			generation,
			target,
			level: DiagnosticLevel::Error,
			message: message.clone(),
			code: None,
			rendered: message,
			relative_spans: Vec::new(),
		}]);
	}

	/// Returns the active successful baseline, if one is installed.
	pub fn baseline(&self) -> Option<&CompiledBaseline> {
		self.baseline.as_ref()
	}

	/// Returns the mutable overlay state for diagnostics and tests.
	pub fn overlays(&self) -> &StaticOverlayStore {
		&self.overlays
	}

	fn start_rebuild(
		&mut self,
		generation: PatchGeneration,
		paths: Vec<PathBuf>,
	) -> DispatchOutcome {
		self.pending = Some(PendingGeneration {
			generation,
			paths,
			patches: Vec::new(),
			fallback_started: true,
		});
		self.notify_build_started(generation);
		DispatchOutcome::RebuildStarted(generation)
	}

	fn allocate_generation(&mut self) -> PatchGeneration {
		self.next_generation = PatchGeneration(self.next_generation.0.saturating_add(1));
		self.next_generation
	}

	fn notify_template_patch(&self, batch: reinhardt_pages::hmr::TemplatePatchBatch) {
		if let Some(server) = &self.hmr_server {
			server.notify_template_patch(batch);
		}
	}

	fn notify_build_started(&self, generation: PatchGeneration) {
		if let Some(server) = &self.hmr_server {
			server.notify_build_started(generation, vec![reinhardt_pages::hmr::BuildTarget::Wasm]);
		}
	}

	fn notify_diagnostics(&self, diagnostics: Vec<BuildDiagnostic>) {
		if let Some(server) = &self.hmr_server {
			server.notify_build_diagnostics(diagnostics);
		}
	}

	fn notify_build_recovered(&self, generation: PatchGeneration) {
		if let Some(server) = &self.hmr_server {
			server.notify_build_recovered(generation);
		}
	}

	fn notify_full_reload(&self, reason: &str) {
		if let Some(server) = &self.hmr_server {
			server.notify_full_reload(reason);
		}
	}
}

fn diagnostic_to_build_diagnostic(
	generation: PatchGeneration,
	diagnostic: crate::TemplateDiagnostic,
) -> BuildDiagnostic {
	let relative_spans = diagnostic
		.relative_span
		.map(|(start, end)| DiagnosticSpan {
			file_name: diagnostic.source_id.0.clone(),
			line_start: 1,
			line_end: 1,
			column_start: start as u32,
			column_end: end as u32,
			is_primary: true,
			label: None,
		})
		.into_iter()
		.collect();
	BuildDiagnostic {
		generation,
		target: DiagnosticTarget::Template,
		level: DiagnosticLevel::Error,
		message: diagnostic.message.clone(),
		code: None,
		rendered: diagnostic.message,
		relative_spans,
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn no_baseline_starts_fallback_build() {
		let server = HmrServer::new(reinhardt_pages::hmr::HmrConfig::builder().build());
		let mut rx = server.sender().subscribe();
		let mut coordinator = TemplateHotReloadCoordinator::new(".", Some(server));
		assert_eq!(
			coordinator.classify_and_dispatch(vec![PathBuf::from("src/page.rs")]),
			DispatchOutcome::RebuildStarted(PatchGeneration(1))
		);
		let message = rx.try_recv().expect("build start message");
		assert!(message.contains("build_started"));
	}

	#[test]
	fn successful_build_requires_matching_baseline_identity() {
		let mut coordinator = TemplateHotReloadCoordinator::new(".", None);
		let artifact = TemplateBuildArtifact {
			build_id: CompiledBuildId([1; 32]),
			manifest_digest: [2; 32],
			baseline: CompiledBaseline::new(CompiledBuildId([9; 32]), [2; 32]),
		};
		assert!(matches!(
			coordinator.install_successful_build(artifact),
			Err(CoordinatorError::BaselineInstall(_))
		));
		assert!(coordinator.baseline().is_none());
	}
}
