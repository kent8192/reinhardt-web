//! Debounced file-system watcher for hot-reload.
//!
//! This module owns the `notify::RecommendedWatcher` and the per-event dispatch
//! loop that fans out to the WASM and server rebuild pipelines. The outer loop
//! is intentionally resilient: pipeline failures are logged but never bubble
//! out as `Err`, so a fix-and-save retry path always exists (spec §6 OL-1).
//!
//! The public surface is `pub(crate)` because callers live inside the same
//! crate (`builtin::Runserver::run_with_autoreload`).

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::Duration;

use notify::Event;
use tokio::sync::mpsc::Receiver;
use tokio::sync::oneshot;
use tokio::time::{Instant, timeout_at};

use crate::CommandContext;
use crate::source_roots::SourceRoots;

/// Time window over which bursts of events are coalesced into a single
/// rebuild trigger. Matches cargo-leptos / dx serve precedent.
pub const DEBOUNCE_WINDOW: Duration = Duration::from_millis(300);

/// Decide whether a `notify::Event` should trigger a rebuild.
///
/// The accept rules are intentionally narrow:
/// * Event kind must be `Modify`, `Create`, or `Remove`.
/// * At least one path must end in `.rs` or `.toml`.
/// * Paths inside `target/` or `.git/`, and editor sidecar files
///   (`~`, `.swp`, `.tmp`), are rejected.
pub fn is_relevant_change(event: &Event) -> bool {
	use notify::EventKind;

	if !matches!(
		event.kind,
		EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)
	) {
		return false;
	}
	event.paths.iter().any(|p| {
		let path_str = p.to_string_lossy();
		!path_str.contains("/target/")
			&& !path_str.contains("/.git/")
			&& !path_str.ends_with('~')
			&& !path_str.ends_with(".swp")
			&& !path_str.ends_with(".tmp")
			&& (path_str.ends_with(".rs") || path_str.ends_with(".toml"))
	})
}

/// Block until a relevant event arrives, then keep collecting any further
/// events that arrive within `window` of *now*.
///
/// Returns the deduplicated, sorted set of paths from all coalesced events,
/// or `None` if the channel closes before any relevant event arrives.
pub async fn debounce_next(rx: &mut Receiver<Event>, window: Duration) -> Option<Vec<PathBuf>> {
	// Phase 1: wait for the first relevant event (drop noise silently).
	let first = loop {
		match rx.recv().await {
			Some(ev) if is_relevant_change(&ev) => break ev,
			Some(_) => continue,
			None => return None,
		}
	};

	let mut paths: BTreeSet<PathBuf> = BTreeSet::new();
	for p in first.paths {
		paths.insert(p);
	}

	// Phase 2: collect any further events arriving before the absolute
	// deadline. Using an absolute deadline (rather than a fresh per-event
	// timeout) bounds total wait time even under sustained event bursts.
	let deadline = Instant::now() + window;
	loop {
		match timeout_at(deadline, rx.recv()).await {
			Ok(Some(ev)) => {
				if is_relevant_change(&ev) {
					for p in ev.paths {
						paths.insert(p);
					}
				}
			}
			Ok(None) => break, // channel closed
			Err(_) => break,   // deadline reached
		}
	}

	Some(paths.into_iter().collect())
}

/// Configuration for `run_watcher`.
pub struct WatcherConfig {
	/// Bin name passed to `cargo build --bin`.
	pub bin_name: String,
	/// Source directories and manifest files to subscribe to.
	pub roots: SourceRoots,
	/// When `true`, suppress the WASM rebuild pipeline.
	pub no_wasm_rebuild: bool,
	/// When `true`, the project has the pages feature enabled.
	#[cfg(feature = "pages")]
	pub pages_enabled: bool,
}

/// Run the hot-reload watcher loop until shutdown.
///
/// The loop handles three concerns:
///
/// 1. Subscribes the recommended `notify` watcher to every existing
///    `roots.src_dirs` (recursively) and `roots.manifest_files`
///    (non-recursively). Non-existent paths are skipped without error.
/// 2. Awaits debounced events, dispatching each to the WASM pipeline
///    (when `pages_enabled && !no_wasm_rebuild`) and then the server
///    pipeline. Pipeline failures are logged but never returned as `Err`.
/// 3. Honours the `shutdown_rx` oneshot: on shutdown the running child
///    is killed and reaped before returning.
///
/// Only watcher infrastructure errors (e.g. `notify::Watcher::new` failure
/// or a failed `watch` subscription on an existing path) propagate as
/// `Err`.
pub async fn run_watcher(
	ctx: &CommandContext,
	config: &WatcherConfig,
	shutdown_rx: oneshot::Receiver<()>,
	mut current_child: tokio::process::Child,
	respawn: impl Fn() -> std::io::Result<tokio::process::Child> + Send + Sync,
) -> Result<(), notify::Error> {
	use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};

	// Bounded channel: notify is bursty and we never want a single backed-up
	// watcher to wedge the runtime. 256 is an order of magnitude above
	// observed IDE save bursts.
	let (tx, mut rx) = tokio::sync::mpsc::channel::<Event>(256);

	let mut watcher = RecommendedWatcher::new(
		move |res: Result<Event, notify::Error>| {
			if let Ok(event) = res {
				// `blocking_send` is fine here: notify dispatches callbacks
				// from a dedicated OS thread, not the tokio runtime.
				let _ = tx.blocking_send(event);
			}
		},
		Config::default(),
	)?;

	// Subscribe each root. Missing paths are skipped (workspace members may
	// be referenced in `Cargo.toml` but not yet exist on disk).
	for dir in &config.roots.src_dirs {
		if dir.exists() {
			watcher.watch(dir, RecursiveMode::Recursive)?;
		}
	}
	for manifest in &config.roots.manifest_files {
		if manifest.exists() {
			watcher.watch(manifest, RecursiveMode::NonRecursive)?;
		}
	}

	let mut shutdown_rx = shutdown_rx;

	loop {
		tokio::select! {
			biased;
			_ = &mut shutdown_rx => {
				let _ = current_child.kill().await;
				let _ = current_child.wait().await;
				return Ok(());
			}
			debounced = debounce_next(&mut rx, DEBOUNCE_WINDOW) => {
				let Some(paths) = debounced else {
					// Channel closed: the watcher dropped or the OS torn
					// the subscription down. Treat as graceful shutdown.
					let _ = current_child.kill().await;
					let _ = current_child.wait().await;
					return Ok(());
				};
				ctx.info(&format!(
					"[hot-reload] change detected ({} path(s)), rebuilding...",
					paths.len()
				));

				// Spec §4: run wasm + server pipelines in parallel. They
				// touch disjoint cargo target directories (`wasm32-unknown-unknown`
				// vs `debug`) and the wasm pipeline does not interact with the
				// running child process, so concurrent execution is safe.
				let wasm_fut = async {
					#[cfg(feature = "pages")]
					{
						if config.pages_enabled && !config.no_wasm_rebuild {
							let outcome =
								crate::wasm_rebuild_pipeline::WasmRebuildPipeline::run(ctx).await;
							if let Some(line) =
								crate::wasm_rebuild_pipeline::WasmRebuildPipeline::format_log_line(
									&outcome,
								) {
								eprintln!("{}", line);
								if matches!(
									outcome,
									crate::wasm_rebuild_pipeline::WasmRebuildOutcome::Failed { .. }
								) {
									eprintln!("[hot-reload] watching for next change...");
								}
							}
						}
					}
				};
				let server_fut = crate::server_rebuild_pipeline::ServerRebuildPipeline::run(
					&config.bin_name,
					&mut current_child,
					&respawn,
				);
				let ((), (_outcome, new_child)) = tokio::join!(wasm_fut, server_fut);
				if let Some(child) = new_child {
					current_child = child;
				}
				// Pipeline failures are recorded as log lines and never
				// propagate as Err — the loop continues unconditionally.
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use notify::event::{CreateKind, ModifyKind, RemoveKind};
	use notify::{Event, EventKind};
	use rstest::rstest;
	use std::path::PathBuf;
	use tokio::sync::mpsc;

	fn ev(kind: EventKind, path: &str) -> Event {
		Event {
			kind,
			paths: vec![PathBuf::from(path)],
			attrs: Default::default(),
		}
	}

	#[rstest]
	#[case::rust_file_modify(EventKind::Modify(ModifyKind::Any), "/project/src/main.rs", true)]
	#[case::toml_file_modify(EventKind::Modify(ModifyKind::Any), "/project/Cargo.toml", true)]
	#[case::rust_file_create(EventKind::Create(CreateKind::File), "/project/src/new.rs", true)]
	#[case::rust_file_remove(EventKind::Remove(RemoveKind::File), "/project/src/old.rs", true)]
	#[case::target_dir_rejected(
		EventKind::Modify(ModifyKind::Any),
		"/project/target/debug/main.rs",
		false
	)]
	#[case::git_dir_rejected(
		EventKind::Modify(ModifyKind::Any),
		"/project/.git/objects/abc",
		false
	)]
	#[case::swap_file_rejected(
		EventKind::Modify(ModifyKind::Any),
		"/project/src/main.rs.swp",
		false
	)]
	#[case::markdown_rejected(EventKind::Modify(ModifyKind::Any), "/project/README.md", false)]
	fn is_relevant_change_filter_cases(
		#[case] kind: EventKind,
		#[case] path: &str,
		#[case] expected: bool,
	) {
		// Arrange
		let event = ev(kind, path);

		// Act
		let actual = is_relevant_change(&event);

		// Assert
		assert_eq!(
			actual, expected,
			"is_relevant_change({path:?}) = {actual}, want {expected}"
		);
	}

	#[tokio::test(flavor = "current_thread", start_paused = true)]
	async fn debounce_coalesces_burst_into_single_trigger() {
		// Arrange: three rapid events, two of them on the same path so the
		// dedup logic also gets exercised.
		let (tx, mut rx) = mpsc::channel::<Event>(8);
		tx.send(ev(EventKind::Modify(ModifyKind::Any), "/p/src/a.rs"))
			.await
			.unwrap();
		tx.send(ev(EventKind::Modify(ModifyKind::Any), "/p/src/a.rs"))
			.await
			.unwrap();
		tx.send(ev(EventKind::Modify(ModifyKind::Any), "/p/src/b.rs"))
			.await
			.unwrap();

		// Act
		let result = debounce_next(&mut rx, Duration::from_millis(300)).await;

		// Assert: returns exactly one Vec containing both unique paths,
		// sorted (BTreeSet ordering).
		assert_eq!(
			result,
			Some(vec![
				PathBuf::from("/p/src/a.rs"),
				PathBuf::from("/p/src/b.rs"),
			]),
		);
	}

	#[tokio::test(flavor = "current_thread", start_paused = true)]
	async fn debounce_returns_none_when_channel_closed_without_events() {
		// Arrange: drop the sender immediately so the channel closes with no
		// pending messages.
		let (tx, mut rx) = mpsc::channel::<Event>(1);
		drop(tx);

		// Act
		let result = debounce_next(&mut rx, Duration::from_millis(300)).await;

		// Assert
		assert!(result.is_none());
	}
}
