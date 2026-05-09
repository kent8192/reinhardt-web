//! Shared `Uuid` / `Option<Uuid>` primary-key shape detection.
//!
//! This module is the single source of truth for asking "does this PK
//! type need the `Uuid::now_v7()` codegen path?". It serves both
//! `user_attribute` (which decides whether to emit a fresh-UUID PK
//! setter for `init_superuser`, see issue #4237) and `model_derive`
//! (which uses `is_uuid_type` while wiring up auto-generated field
//! defaults). Centralising the detection here prevents the two macros
//! from drifting when we add support for additional UUID wrappers or
//! alternative path patterns. See issue #4246.

use syn::Type;

/// Inspect a syntactic type and report whether it is a `Uuid` (or an
/// `Option<Uuid>`) primary key.
///
/// Returns `(is_uuid, is_option)`:
/// - `is_uuid` is `true` when the (optionally `Option`-wrapped) type
///   has a final path segment of `Uuid`. Both bare `Uuid` and
///   fully-qualified `uuid::Uuid` resolve to `true`.
/// - `is_option` is `true` when the outer type's last segment is
///   `Option`, regardless of the inner type.
///
/// Detection is deliberately last-segment only: the macros never see
/// the resolved type, so `MyUuid` or `UuidV4` correctly report
/// `is_uuid = false`.
pub(crate) fn pk_uuid_shape(ty: &Type) -> (bool, bool) {
	fn last_segment_is(ty: &Type, name: &str) -> bool {
		matches!(ty, Type::Path(p) if p.path.segments.last().is_some_and(|s| s.ident == name))
	}
	if let Type::Path(type_path) = ty
		&& let Some(last_segment) = type_path.path.segments.last()
		&& last_segment.ident == "Option"
		&& let syn::PathArguments::AngleBracketed(args) = &last_segment.arguments
		&& let Some(syn::GenericArgument::Type(inner)) = args.args.first()
	{
		return (last_segment_is(inner, "Uuid"), true);
	}
	(last_segment_is(ty, "Uuid"), false)
}

#[cfg(test)]
mod tests {
	use super::pk_uuid_shape;
	use syn::parse_quote;

	// Regression coverage migrated from `user_attribute.rs` for issue
	// #4237: the macro decides whether to emit a `Uuid::now_v7()` PK
	// setter based on `pk_uuid_shape`. Each case below corresponds to a
	// real-world PK declaration we expect users to write in
	// `#[user(full = true)] #[model(...)] ...` types.

	#[test]
	fn pk_uuid_shape_detects_bare_uuid() {
		// Arrange / Act
		let ty: syn::Type = parse_quote!(Uuid);

		// Assert — bare `Uuid` is the canonical superuser-PK shape.
		assert_eq!(pk_uuid_shape(&ty), (true, false));
	}

	#[test]
	fn pk_uuid_shape_detects_qualified_uuid_path() {
		// Arrange / Act
		let ty: syn::Type = parse_quote!(uuid::Uuid);

		// Assert — fully-qualified path must still resolve, otherwise
		// users who write `pub id: uuid::Uuid` would silently slip
		// back into the nil-UUID bug.
		assert_eq!(pk_uuid_shape(&ty), (true, false));
	}

	#[test]
	fn pk_uuid_shape_detects_option_uuid() {
		// Arrange / Act
		let ty: syn::Type = parse_quote!(Option<Uuid>);

		// Assert — `Option<Uuid>` is uncommon for a PK but legal; the
		// macro must wrap the seed value in `Some(now_v7())` to keep
		// type-checking happy.
		assert_eq!(pk_uuid_shape(&ty), (true, true));
	}

	#[test]
	fn pk_uuid_shape_detects_option_qualified_uuid() {
		// Arrange / Act
		let ty: syn::Type = parse_quote!(Option<uuid::Uuid>);

		// Assert — Option around a fully-qualified Uuid still counts.
		assert_eq!(pk_uuid_shape(&ty), (true, true));
	}

	#[test]
	fn pk_uuid_shape_skips_integer_pk() {
		// Arrange / Act
		let ty: syn::Type = parse_quote!(i64);

		// Assert — integer PK types must NOT receive the `now_v7()`
		// assignment; the existing `Self::default()` path is correct
		// for non-Uuid PKs and the macro must not corrupt that.
		assert_eq!(pk_uuid_shape(&ty), (false, false));
	}

	#[test]
	fn pk_uuid_shape_skips_string_pk() {
		// Arrange / Act
		let ty: syn::Type = parse_quote!(String);

		// Assert — string PK (e.g. natural keys) is also untouched.
		assert_eq!(pk_uuid_shape(&ty), (false, false));
	}

	#[test]
	fn pk_uuid_shape_skips_option_non_uuid() {
		// Arrange / Act
		let ty: syn::Type = parse_quote!(Option<i64>);

		// Assert — Option of a non-Uuid scalar must report
		// `is_uuid = false`, so the macro emits no setter.
		assert_eq!(pk_uuid_shape(&ty), (false, true));
	}

	#[test]
	fn pk_uuid_shape_does_not_match_lookalike_named_types() {
		// Arrange / Act — `MyUuid` shares a substring with `Uuid` but is
		// a different identifier. The helper compares whole identifiers
		// (not substrings), so it must not falsely seed a v7 UUID into
		// an unrelated user-defined PK type.
		let aliased: syn::Type = parse_quote!(MyUuid);
		let suffixed: syn::Type = parse_quote!(UuidV4);

		// Assert — neither lookalike resolves to `Uuid`, so the macro
		// will skip the `now_v7()` setter (and the user's existing
		// `Default` for the type takes effect, just as before).
		assert_eq!(pk_uuid_shape(&aliased), (false, false));
		assert_eq!(pk_uuid_shape(&suffixed), (false, false));
	}
}
