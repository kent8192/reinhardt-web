/// Manual action registration support
/// This provides a simpler API for registering actions without macros
use crate::viewsets::metadata::{ActionMetadata, FunctionActionHandler, get_actions_for_viewset};
use reinhardt_http::{Request, Response, Result};
use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, RwLock};
use tracing;

/// Global action registry for manual registration
/// This is a temporary solution until the procedural macro is fully implemented
pub struct ManualActionRegistry {
	actions: RwLock<HashMap<String, Vec<ActionMetadata>>>,
}

impl ManualActionRegistry {
	fn new() -> Self {
		Self {
			actions: RwLock::new(HashMap::new()),
		}
	}

	/// Register an action for a ViewSet type
	pub fn register(&self, viewset_type: &str, action: ActionMetadata) {
		let mut actions = self.actions.write().unwrap_or_else(|e| {
			tracing::warn!("Action registry RwLock was poisoned, recovering");
			e.into_inner()
		});
		actions
			.entry(viewset_type.to_string())
			.or_default()
			.push(action);
	}

	/// Get actions for a ViewSet type
	pub fn get_actions(&self, viewset_type: &str) -> Vec<ActionMetadata> {
		let actions = self.actions.read().unwrap_or_else(|e| {
			tracing::warn!("Action registry RwLock was poisoned, recovering");
			e.into_inner()
		});
		actions.get(viewset_type).cloned().unwrap_or_default()
	}

	/// Clear all registered actions (for testing)
	pub fn clear(&self) {
		let mut actions = self.actions.write().unwrap_or_else(|e| {
			tracing::warn!("Action registry RwLock was poisoned, recovering");
			e.into_inner()
		});
		actions.clear();
	}
}

lazy_static::lazy_static! {
	static ref GLOBAL_REGISTRY: ManualActionRegistry = ManualActionRegistry::new();
}

/// Register an action manually
pub fn register_action(viewset_type: &str, action: ActionMetadata) {
	GLOBAL_REGISTRY.register(viewset_type, action);
}

/// Get registered actions for a ViewSet type
pub fn get_registered_actions(viewset_type: &str) -> Vec<ActionMetadata> {
	GLOBAL_REGISTRY.get_actions(viewset_type)
}

/// Clear all registered actions (primarily for testing)
pub fn clear_actions() {
	GLOBAL_REGISTRY.clear();
}

/// Bridge marker-keyed action submissions into the runtime-ViewSet-keyed
/// registry.
///
/// The impl-form of `#[viewset(basename = "...")]` (Issue #4507, Phase 5.1)
/// emits a `#[ctor::ctor]` startup function that calls `register_action`
/// keyed by the marker type's `std::any::type_name` — e.g.
/// `type_name::<SnippetViewSet>()`. At runtime the `ViewSet` trait's
/// `get_extra_actions()` queries the registry by the *concrete* ViewSet's
/// type name (e.g. `ModelViewSet<Snippet, ...>`), so the marker-keyed
/// entries must be re-registered under the runtime key before the
/// dispatcher can find them.
///
/// `viewset_with_actions::<V, M>(...)` calls this helper after inserting
/// the ViewSet into the router, passing the marker's `type_name` as
/// `marker_type` and the concrete ViewSet's `type_name` as `viewset_type`.
/// The bridge:
///
/// 1. Reads every action registered under `marker_type` (from both the
///    manual registry — populated by the `#[ctor]` emitter — and the
///    `inventory` static collection — for forward compatibility once
///    `const_type_name` stabilizes).
/// 2. Re-registers each `ActionMetadata` under `viewset_type` in the
///    manual registry.
///
/// The helper short-circuits when `marker_type == viewset_type` to avoid
/// duplicating entries when the marker happens to BE the runtime ViewSet.
///
/// Refs Issue #4507.
pub fn bridge_marker_actions_to_viewset(marker_type: &str, viewset_type: &str) {
	if marker_type == viewset_type {
		// No-op: the marker IS the runtime ViewSet (rare but valid).
		return;
	}

	// Dedupe by `(url_name, name, detail)` against the destination slot so
	// repeated bridge calls (multiple `viewset_with_actions::<V, M>(...)`
	// mounts, repeated `register_router` runs, or test setups that rebuild
	// the router) do not append duplicate `ActionMetadata` entries. The
	// underlying `ManualActionRegistry::register` appends without checking,
	// and downstream routers warn on duplicate route names and keep only
	// the first registration in the reverser — silently dropping later
	// updates. Deduping here keeps the destination slot canonical.
	let existing: std::collections::HashSet<(Option<String>, String, bool)> =
		get_registered_actions(viewset_type)
			.into_iter()
			.map(|a| (a.url_name.clone(), a.name.clone(), a.detail))
			.collect();

	let mut already_bridged = existing;

	let mut bridge_one = |action: ActionMetadata| {
		let key = (action.url_name.clone(), action.name.clone(), action.detail);
		if already_bridged.insert(key) {
			register_action(viewset_type, action);
		}
	};

	// Manual-registry side (populated by the impl-form `#[viewset]` macro's
	// `#[ctor]` startup hook).
	for action in get_registered_actions(marker_type) {
		bridge_one(action);
	}
	// Inventory side (forward-compatibility for once `const_type_name`
	// stabilizes — currently empty for marker-keyed submissions but the
	// fan-out is a few cycles and keeps the contract stable).
	for action in get_actions_for_viewset(marker_type) {
		bridge_one(action);
	}
}

/// Helper to create an action with a closure
pub fn action<F, Fut>(name: impl Into<String>, detail: bool, handler: F) -> ActionMetadata
where
	F: Fn(Request) -> Fut + Send + Sync + 'static,
	Fut: Future<Output = Result<Response>> + Send + 'static,
{
	let handler_fn = Arc::new(handler);
	ActionMetadata::new(name)
		.with_detail(detail)
		.with_handler(FunctionActionHandler::new(move |req| {
			let h = handler_fn.clone();
			Box::pin(async move { h(req).await })
		}))
}

/// Macro to simplify action registration
#[macro_export]
macro_rules! register_viewset_actions {
    ($viewset_type:ty => {
        $(
            $action_name:ident ( $detail:expr $(, $attr:ident = $value:expr )* ) => $handler:expr
        ),* $(,)?
    }) => {
        {
            let viewset_type = std::any::type_name::<$viewset_type>();
            $(
                let mut action = $crate::viewsets::metadata::ActionMetadata::new(stringify!($action_name))
                    .with_detail($detail);

                $(
                    action = register_viewset_actions!(@attr action, $attr, $value);
                )*

                let handler = $handler;
                action = action.with_handler($crate::viewsets::metadata::FunctionActionHandler::new(
                    move |req| Box::pin(handler(req))
                ));

                $crate::viewsets::registry::register_action(viewset_type, action);
            )*
        }
    };

    (@attr $action:expr, name, $value:expr) => {
        $action.with_custom_name($value)
    };
    (@attr $action:expr, suffix, $value:expr) => {
        $action.with_suffix($value)
    };
    (@attr $action:expr, url_path, $value:expr) => {
        $action.with_url_path($value)
    };
    (@attr $action:expr, url_name, $value:expr) => {
        $action.with_url_name($value)
    };
    (@attr $action:expr, methods, $value:expr) => {
        $action.with_methods($value)
    };
}
