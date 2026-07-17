//! Document-level SPA link interceptor for `ClientLauncher::launch`.

#[cfg(wasm)]
use super::with_spa_router;

#[cfg(wasm)]
use crate::router::PrefetchMode;

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

/// Owns all document-level link listeners installed by the launcher.
#[cfg(wasm)]
pub(super) struct LinkInterceptorGuard {
	document: web_sys::Document,
	click: wasm_bindgen::closure::Closure<dyn FnMut(web_sys::MouseEvent)>,
	pointerover: wasm_bindgen::closure::Closure<dyn FnMut(web_sys::PointerEvent)>,
	focusin: wasm_bindgen::closure::Closure<dyn FnMut(web_sys::FocusEvent)>,
	viewport_observer: std::cell::RefCell<Option<ViewportPrefetchObserver>>,
}

#[cfg(wasm)]
struct ViewportPrefetchObserver {
	observer: web_sys::IntersectionObserver,
	// Retained so the observer callback remains alive until the observer drops.
	_callback:
		wasm_bindgen::closure::Closure<dyn FnMut(js_sys::Array, web_sys::IntersectionObserver)>,
}

#[cfg(wasm)]
impl Drop for LinkInterceptorGuard {
	fn drop(&mut self) {
		use wasm_bindgen::JsCast;
		let _ = self
			.document
			.remove_event_listener_with_callback("click", self.click.as_ref().unchecked_ref());
		let _ = self.document.remove_event_listener_with_callback(
			"pointerover",
			self.pointerover.as_ref().unchecked_ref(),
		);
		let _ = self
			.document
			.remove_event_listener_with_callback("focusin", self.focusin.as_ref().unchecked_ref());
		if let Some(observer) = self.viewport_observer.get_mut().take() {
			observer.observer.disconnect();
		}
	}
}

#[cfg(wasm)]
impl LinkInterceptorGuard {
	/// Starts viewport observation only after mounted links request it.
	pub(super) fn observe_viewport_prefetch_links(&self) {
		use wasm_bindgen::JsCast;
		use wasm_bindgen::closure::Closure;

		let Ok(viewport_links) = self
			.document
			.query_selector_all("[data-prefetch=\"viewport\"]")
		else {
			return;
		};
		if viewport_links.length() == 0 {
			return;
		}

		let mut slot = self.viewport_observer.borrow_mut();
		if slot.is_none() {
			let callback = Closure::wrap(Box::new(
				move |entries: js_sys::Array, _observer: web_sys::IntersectionObserver| {
					for entry in entries.iter() {
						if let Ok(entry) = entry.dyn_into::<web_sys::IntersectionObserverEntry>()
							&& entry.is_intersecting()
						{
							prefetch_from_target(Some(entry.target().into()));
						}
					}
				},
			) as Box<dyn FnMut(_, _)>);
			let Ok(observer) =
				web_sys::IntersectionObserver::new(callback.as_ref().unchecked_ref())
			else {
				return;
			};
			*slot = Some(ViewportPrefetchObserver {
				observer,
				_callback: callback,
			});
		}

		let observer = &slot
			.as_ref()
			.expect("viewport observer is installed when viewport links exist")
			.observer;
		for index in 0..viewport_links.length() {
			if let Some(node) = viewport_links.item(index)
				&& let Ok(element) = node.dyn_into::<web_sys::Element>()
			{
				observer.observe(&element);
			}
		}
	}
}

#[cfg(wasm)]
fn anchor_from_target(target: Option<web_sys::EventTarget>) -> Option<web_sys::Element> {
	use wasm_bindgen::JsCast;
	let target = target?;
	let mut element = target.dyn_ref::<web_sys::Element>().cloned();
	if element.is_none()
		&& let Some(node) = target.dyn_ref::<web_sys::Node>()
	{
		let mut current = node.parent_node();
		while let Some(node) = current {
			if let Some(candidate) = node.dyn_ref::<web_sys::Element>() {
				element = Some(candidate.clone());
				break;
			}
			current = node.parent_node();
		}
	}
	while let Some(ref candidate) = element {
		if candidate.tag_name().eq_ignore_ascii_case("A") {
			return element;
		}
		element = candidate.parent_element();
	}
	None
}

#[cfg(wasm)]
fn prefetch_from_target(target: Option<web_sys::EventTarget>) {
	let Some(anchor) = anchor_from_target(target) else {
		return;
	};
	let Some(href) = anchor.get_attribute("href") else {
		return;
	};
	let mode = match anchor.get_attribute("data-prefetch").as_deref() {
		Some("hover") => PrefetchMode::Hover,
		Some("viewport") => PrefetchMode::Viewport,
		_ => PrefetchMode::None,
	};
	if mode == PrefetchMode::None || !href.starts_with('/') || href.starts_with("//") {
		return;
	}
	let _ = crate::app::try_with_navigation_coordinator(|coordinator| {
		let _ = coordinator.prefetch(href);
	});
}

/// Install document-level click and prefetch listeners.
/// `<a href="/...">` anchors into `Router::push` navigations.
///
/// Skips external links, `target="_blank"`, `download`, `rel="external"`,
/// and modifier-key clicks (so the user can still open in a new tab).
///
#[cfg(wasm)]
pub(super) fn install_link_interceptor(
	document: &web_sys::Document,
) -> Result<LinkInterceptorGuard, wasm_bindgen::JsValue> {
	use wasm_bindgen::JsCast;
	use wasm_bindgen::closure::Closure;

	let click = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
		// Walk up the DOM looking for the closest <a> ancestor.
		//
		// `event.target()` may be a `Node` that is not itself an `Element`
		// — most commonly a `Text` node when the user clicks on the link
		// label, but also `Comment`, `DocumentFragment`, etc. Casting
		// straight to `Element` (as the previous implementation did)
		// silently no-op'd in that case and broke SPA navigation for the
		// common case of `<a href="/x">label</a>` (Refs #4330). Promote a
		// non-Element `Node` target to its nearest `Element` ancestor via
		// `parent_node()` first, then continue the existing tag-name walk
		// to find the enclosing `<a>`.
		let Some(target) = event.target() else {
			return;
		};
		let mut el: Option<web_sys::Element> = target.dyn_ref::<web_sys::Element>().cloned();
		if el.is_none()
			&& let Some(node) = target.dyn_ref::<web_sys::Node>()
		{
			let mut current: Option<web_sys::Node> = node.parent_node();
			while let Some(ref n) = current {
				if let Some(e) = n.dyn_ref::<web_sys::Element>() {
					el = Some(e.clone());
					break;
				}
				current = n.parent_node();
			}
		}
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
		let replace = anchor
			.get_attribute("data-replace")
			.is_some_and(|value| value.eq_ignore_ascii_case("true"));
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

		// Surface `Router::push` failures instead of silently swallowing
		// them with `let _ = r.push(href);`. The `nav_diag!` console
		// line is debug-gated (cheap, no rebuild required), while the
		// `console.warn` below fires in all builds so SPA navigation
		// regressions are visible in production WASM bundles without
		// requiring a tracing subscriber. The `tracing` crate is not
		// pulled in on the wasm32 target (see Cargo.toml), so we use
		// `web_sys::console::warn_1` directly here (Refs #4331).
		let result = if replace {
			crate::reactive::hooks::RouterHandle.replace(href)
		} else {
			crate::reactive::hooks::RouterHandle.push(href)
		};
		match result {
			Ok(()) => {}
			Err(err) => {
				crate::nav_diag!(
					"site=link_interceptor push_failed href={} error={}",
					href,
					err,
				);
				::web_sys::console::warn_1(
					&format!(
						"reinhardt-pages: SPA link interceptor: Router::push failed: href={} error={}",
						href, err
					)
					.into(),
				);
			}
		}
	}) as Box<dyn FnMut(_)>);

	document.add_event_listener_with_callback("click", click.as_ref().unchecked_ref())?;
	let pointerover = Closure::wrap(Box::new(move |event: web_sys::PointerEvent| {
		prefetch_from_target(event.target());
	}) as Box<dyn FnMut(_)>);
	document
		.add_event_listener_with_callback("pointerover", pointerover.as_ref().unchecked_ref())?;
	let focusin = Closure::wrap(Box::new(move |event: web_sys::FocusEvent| {
		prefetch_from_target(event.target());
	}) as Box<dyn FnMut(_)>);
	document.add_event_listener_with_callback("focusin", focusin.as_ref().unchecked_ref())?;
	Ok(LinkInterceptorGuard {
		document: document.clone(),
		click,
		pointerover,
		focusin,
		viewport_observer: std::cell::RefCell::new(None),
	})
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
