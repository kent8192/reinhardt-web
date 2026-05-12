//! Document-level SPA link interceptor for `ClientLauncher::launch`.

#[cfg(wasm)]
use super::with_spa_router;

/// Anchor attributes relevant to the link interceptor decision.
///
/// Extracted into a plain struct so the decision logic in
/// [`should_intercept`] stays a pure function and can be unit-tested on
/// the host without a real DOM.
#[cfg_attr(not(any(wasm, test)), allow(dead_code))]
pub(super) struct AnchorAttrs<'a> {
	pub(super) has_modifier_key: bool,
	pub(super) href: Option<&'a str>,
	pub(super) target: Option<&'a str>,
	pub(super) has_download: bool,
	pub(super) rel: Option<&'a str>,
}

/// Decide whether the link interceptor should hijack a click.
///
/// Returns `Some(href)` if the click should be turned into a SPA push,
/// or `None` to let the browser handle the click normally.
#[cfg_attr(not(any(wasm, test)), allow(dead_code))]
pub(super) fn should_intercept<'a>(attrs: &AnchorAttrs<'a>) -> Option<&'a str> {
	if attrs.has_modifier_key {
		return None;
	}
	let href = attrs.href?;
	// Internal link: starts with `/` but not `//` (protocol-relative URLs are
	// treated as external by the browser).
	if !href.starts_with('/') || href.starts_with("//") {
		return None;
	}
	// HTML keyword targets (`_blank`, `_self`, ...) are matched
	// case-insensitively by browsers, so `_BLANK` must also bypass
	// SPA interception.
	if let Some(target) = attrs.target
		&& target.eq_ignore_ascii_case("_blank")
	{
		return None;
	}
	if attrs.has_download {
		return None;
	}
	if let Some(rel) = attrs.rel
		&& rel
			.split_ascii_whitespace()
			.any(|w| w.eq_ignore_ascii_case("external"))
	{
		return None;
	}
	Some(href)
}

/// Install a document-level click listener that converts clicks on internal
/// `<a href="/...">` anchors into `Router::push` navigations.
///
/// Skips external links, `target="_blank"`, `download`, `rel="external"`,
/// and modifier-key clicks (so the user can still open in a new tab).
///
/// The closure is leaked via `closure.forget()` so the listener lives for
/// the entire WASM module lifetime — same posture as `setup_popstate_listener`.
#[cfg(wasm)]
pub(super) fn install_link_interceptor(
	document: &web_sys::Document,
) -> Result<(), wasm_bindgen::JsValue> {
	use wasm_bindgen::JsCast;
	use wasm_bindgen::closure::Closure;

	let closure = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
		// Walk up the DOM looking for the closest <a> ancestor.
		let Some(target) = event.target() else {
			return;
		};
		let mut el: Option<web_sys::Element> = target.dyn_ref::<web_sys::Element>().cloned();
		while let Some(ref e) = el {
			if e.tag_name().eq_ignore_ascii_case("A") {
				break;
			}
			el = e.parent_element();
		}
		let Some(anchor) = el else {
			return;
		};

		let href = anchor.get_attribute("href");
		let target_attr = anchor.get_attribute("target");
		let rel_attr = anchor.get_attribute("rel");
		let attrs = AnchorAttrs {
			has_modifier_key: event.ctrl_key() || event.meta_key() || event.shift_key(),
			href: href.as_deref(),
			target: target_attr.as_deref(),
			has_download: anchor.has_attribute("download"),
			rel: rel_attr.as_deref(),
		};

		let Some(href) = should_intercept(&attrs) else {
			return;
		};

		event.prevent_default();

		// Opt-in DOM-based diagnostic. Off by default (zero overhead in
		// release WASM); enabled via `pages-nav-diag-dom` feature for
		// SPA navigation regression debugging (Refs #4221).
		crate::nav_diag_dom!("link_interceptor");

		// Diagnostic snapshot of the click-time router state. Gated on
		// debug builds so release WASM does not run the extra
		// `match_path` lookup (Refs #4203).
		#[cfg(debug_assertions)]
		{
			let (router_id, match_some, match_name) = with_spa_router(|r| {
				let m = r.match_path(href);
				(
					r.__diag_router_id(),
					m.is_some(),
					m.as_ref()
						.and_then(|rm| rm.name.clone())
						.unwrap_or_default(),
				)
			});
			crate::nav_diag!(
				"site=link_interceptor router_id={} href={} match_some={} match_name={}",
				router_id,
				href,
				match_some,
				match_name
			);
		}

		with_spa_router(|r| {
			let _ = r.push(href);
		});
	}) as Box<dyn FnMut(_)>);

	document.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())?;
	closure.forget();
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::*;

	fn attrs(href: Option<&str>) -> AnchorAttrs<'_> {
		AnchorAttrs {
			has_modifier_key: false,
			href,
			target: None,
			has_download: false,
			rel: None,
		}
	}

	#[rstest]
	fn test_should_intercept_internal_root_relative_link() {
		// Arrange
		let a = attrs(Some("/users/"));
		// Act
		let result = should_intercept(&a);
		// Assert
		assert_eq!(result, Some("/users/"));
	}

	#[rstest]
	fn test_should_intercept_skips_external_url() {
		// Arrange
		let a = attrs(Some("https://example.com/page"));
		// Act / Assert
		assert_eq!(should_intercept(&a), None);
	}

	#[rstest]
	fn test_should_intercept_skips_protocol_relative_url() {
		// Arrange
		let a = attrs(Some("//example.com/page"));
		// Act / Assert
		assert_eq!(should_intercept(&a), None);
	}

	#[rstest]
	fn test_should_intercept_skips_anchor_without_href() {
		// Arrange
		let a = attrs(None);
		// Act / Assert
		assert_eq!(should_intercept(&a), None);
	}

	#[rstest]
	fn test_should_intercept_skips_relative_link() {
		// Arrange
		let a = attrs(Some("relative/path"));
		// Act / Assert
		assert_eq!(should_intercept(&a), None);
	}

	#[rstest]
	fn test_should_intercept_skips_target_blank() {
		// Arrange
		let mut a = attrs(Some("/users/"));
		a.target = Some("_blank");
		// Act / Assert
		assert_eq!(should_intercept(&a), None);
	}

	#[rstest]
	fn test_should_intercept_skips_target_blank_uppercase() {
		// Arrange
		let mut a = attrs(Some("/users/"));
		a.target = Some("_BLANK");
		// Act / Assert
		assert_eq!(should_intercept(&a), None);
	}

	#[rstest]
	fn test_should_intercept_skips_target_blank_mixed_case() {
		// Arrange
		let mut a = attrs(Some("/users/"));
		a.target = Some("_Blank");
		// Act / Assert
		assert_eq!(should_intercept(&a), None);
	}

	#[rstest]
	fn test_should_intercept_allows_target_self() {
		// Arrange
		let mut a = attrs(Some("/users/"));
		a.target = Some("_self");
		// Act / Assert
		assert_eq!(should_intercept(&a), Some("/users/"));
	}

	#[rstest]
	fn test_should_intercept_skips_download_attribute() {
		// Arrange
		let mut a = attrs(Some("/files/report.pdf"));
		a.has_download = true;
		// Act / Assert
		assert_eq!(should_intercept(&a), None);
	}

	#[rstest]
	fn test_should_intercept_skips_rel_external() {
		// Arrange
		let mut a = attrs(Some("/users/"));
		a.rel = Some("external");
		// Act / Assert
		assert_eq!(should_intercept(&a), None);
	}

	#[rstest]
	fn test_should_intercept_skips_compound_rel_with_external() {
		// Arrange
		let mut a = attrs(Some("/users/"));
		a.rel = Some("noopener external");
		// Act / Assert
		assert_eq!(should_intercept(&a), None);
	}

	#[rstest]
	fn test_should_intercept_is_case_insensitive_for_rel() {
		// Arrange
		let mut a = attrs(Some("/users/"));
		a.rel = Some("EXTERNAL");
		// Act / Assert
		assert_eq!(should_intercept(&a), None);
	}

	#[rstest]
	fn test_should_intercept_allows_other_rel_values() {
		// Arrange
		let mut a = attrs(Some("/users/"));
		a.rel = Some("noopener noreferrer");
		// Act / Assert
		assert_eq!(should_intercept(&a), Some("/users/"));
	}

	#[rstest]
	fn test_should_intercept_skips_modifier_key_click() {
		// Arrange
		let mut a = attrs(Some("/users/"));
		a.has_modifier_key = true;
		// Act / Assert
		assert_eq!(should_intercept(&a), None);
	}
}
