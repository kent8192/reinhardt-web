//! External-consumer compile coverage for the `#[non_exhaustive]` sequence types.
//!
//! The pass case proves that downstream crates can still construct every
//! existing `SequenceOption`/`OwnedBy` variant, build a `SequenceDef`, and match
//! with a wildcard arm. The fail cases prove that exhaustive matches without a
//! wildcard arm are rejected even when they list every current variant.

#[test]
fn sequence_types_require_wildcard_matches_downstream() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/sequence_non_exhaustive/pass/sequence_wildcard_match.rs");
	t.compile_fail("tests/ui/sequence_non_exhaustive/fail/sequence_option_exhaustive_match.rs");
	t.compile_fail("tests/ui/sequence_non_exhaustive/fail/owned_by_exhaustive_match.rs");
}
