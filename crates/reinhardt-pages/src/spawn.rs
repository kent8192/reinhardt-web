//! Deprecated task-spawning module (Issue #4365).
//!
//! The implementation now lives in [`crate::platform`]. This module is kept
//! as a thin deprecation shim so external code importing
//! `reinhardt_pages::spawn::{spawn_task, defer_yield}` continues to compile
//! with a deprecation warning.
//!
//! Migration: replace `reinhardt_pages::spawn::*` with
//! `reinhardt_pages::platform::*` (or use `reinhardt_pages::prelude::*`,
//! which re-exports both under their canonical names).

use std::future::Future;

/// Deprecated alias for [`crate::platform::spawn_task`].
#[deprecated(note = "use `reinhardt_pages::platform::spawn_task` (or the prelude) instead")]
pub fn spawn_task<F>(fut: F)
where
	F: Future<Output = ()> + 'static,
{
	crate::platform::spawn_task(fut)
}

/// Deprecated alias for [`crate::platform::defer_yield`].
#[deprecated(note = "use `reinhardt_pages::platform::defer_yield` (or the prelude) instead")]
pub async fn defer_yield() {
	crate::platform::defer_yield().await
}
