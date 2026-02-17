//! Task spawning utilities for WASM and non-WASM environments.

use std::future::Future;

/// Spawns a task on the current runtime.
///
/// On WASM, this uses `wasm_bindgen_futures::spawn_local` to schedule
/// the task on the browser's event loop. On non-WASM targets, this is
/// a no-op stub.
///
/// # Example
///
/// ```ignore
/// use reinhardt_pages::spawn::spawn_task;
///
/// spawn_task(async move {
///     let data = fetch_data().await;
///     process(data);
/// });
/// ```
#[cfg(target_arch = "wasm32")]
pub fn spawn_task<F>(fut: F)
where
	F: Future<Output = ()> + 'static,
{
	wasm_bindgen_futures::spawn_local(fut);
}

/// No-op stub for non-WASM targets.
#[cfg(not(target_arch = "wasm32"))]
pub fn spawn_task<F>(_fut: F)
where
	F: Future<Output = ()> + 'static,
{
	// Non-WASM: just drop the future
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_spawn_task_compiles() {
		spawn_task(async {
			// テスト処理
		});
	}
}
