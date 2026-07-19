#![cfg(feature = "pages")]

use reinhardt_commands::{
	CompiledBaseline, DispatchOutcome, TemplateHotReloadCoordinator, normalize_build_diagnostics,
};
use reinhardt_pages::hmr::{CompiledBuildId, HmrConfig, HmrServer, PatchGeneration};
use serde_json::json;

#[test]
fn coordinator_without_baseline_requests_rebuild_and_keeps_state_empty() {
	let server = HmrServer::new(HmrConfig::builder().build());
	let mut rx = server.sender().subscribe();
	let mut coordinator = TemplateHotReloadCoordinator::new(".", Some(server));

	let outcome = coordinator.classify_and_dispatch(vec!["src/client.rs".into()]);

	assert_eq!(outcome, DispatchOutcome::RebuildStarted(PatchGeneration(1)));
	assert!(
		rx.try_recv()
			.expect("build lifecycle message")
			.contains("build_started")
	);
	assert!(coordinator.baseline().is_none());
	assert!(coordinator.overlays().overlays.is_empty());
}

#[test]
fn coordinator_rejects_an_artifact_with_a_mismatched_baseline() {
	let mut coordinator = TemplateHotReloadCoordinator::new(".", None);
	let artifact = reinhardt_commands::TemplateBuildArtifact {
		build_id: CompiledBuildId([1; 32]),
		manifest_digest: [2; 32],
		baseline: CompiledBaseline::new(CompiledBuildId([3; 32]), [2; 32]),
	};

	let error = coordinator
		.install_successful_build(artifact)
		.expect_err("identity mismatch must not install a baseline");

	assert!(matches!(
		error,
		reinhardt_commands::CoordinatorError::BaselineInstall(_)
	));
}

#[test]
fn diagnostics_are_relative_ansi_free_and_drop_outside_paths() {
	let message = json!({
		"reason": "compiler-message",
		"package_id": "reinhardt-pages 0.3.2",
		"target": {"name": "frontend-wasm"},
		"message": {
			"message": "\u{001b}[31merror\u{001b}[0m: invalid page",
			"level": "error",
			"code": {"code": "E0001"},
			"rendered": "\u{001b}[31merror[E0001]\u{001b}[0m: invalid page",
			"spans": [
				{"file_name": "/workspace/project/src/page.rs", "line_start": 3, "line_end": 3, "column_start": 2, "column_end": 5, "is_primary": true, "label": "here"},
				{"file_name": "/tmp/secret.rs", "line_start": 1, "line_end": 1, "column_start": 1, "column_end": 1, "is_primary": false}
			]
		}
	}).to_string();

	let diagnostics = normalize_build_diagnostics(
		std::path::Path::new("/workspace/project"),
		PatchGeneration(7),
		&message,
	);

	assert_eq!(diagnostics.len(), 1);
	let diagnostic = &diagnostics[0];
	assert_eq!(diagnostic.generation, PatchGeneration(7));
	assert_eq!(
		diagnostic.target,
		reinhardt_pages::hmr::DiagnosticTarget::WasmRustc
	);
	assert_eq!(diagnostic.code.as_deref(), Some("E0001"));
	assert!(!diagnostic.message.contains('\u{1b}'));
	assert_eq!(diagnostic.relative_spans.len(), 1);
	assert_eq!(diagnostic.relative_spans[0].file_name, "src/page.rs");
}
