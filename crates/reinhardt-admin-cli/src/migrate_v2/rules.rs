//! Registry of all rewriter rules.

pub mod bare_ident;
pub mod component_props;
pub mod use_effect_deps;
pub mod watch_unwrap;

use crate::migrate_v2::rewriter::FileRewriter;

/// Returns all rules in deterministic order.
pub fn all() -> Vec<Box<dyn FileRewriter>> {
	vec![
		Box::new(bare_ident::Rule),
		Box::new(watch_unwrap::Rule),
		Box::new(use_effect_deps::Rule),
		Box::new(component_props::Rule),
	]
}
