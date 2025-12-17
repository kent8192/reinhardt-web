//! Pagination component

use dominator::{Dom, clone, events, html};
use futures_signals::signal::{Mutable, SignalExt};
use std::sync::Arc;
use wasm_bindgen::JsCast;

/// Pagination state
pub struct PaginationState {
	pub current_page: Mutable<u64>,
	pub total_pages: Mutable<u64>,
	pub page_size: Mutable<u64>,
	pub total_items: Mutable<u64>,
}

impl PaginationState {
	/// Create a new pagination state
	pub fn new() -> Arc<Self> {
		Arc::new(Self {
			current_page: Mutable::new(1),
			total_pages: Mutable::new(0),
			page_size: Mutable::new(25),
			total_items: Mutable::new(0),
		})
	}
}

/// Pagination component props
pub struct PaginationProps {
	pub state: Arc<PaginationState>,
	pub on_page_change: Arc<dyn Fn(u64)>,
	pub on_page_size_change: Option<Arc<dyn Fn(u64)>>,
}

/// Render pagination controls
pub fn render(props: PaginationProps) -> Dom {
	let PaginationProps {
		state,
		on_page_change,
		on_page_size_change,
	} = props;

	html!("div", {
		.class("pagination")
		.children(&mut [
			// Page navigation buttons
			html!("div", {
				.class("pagination-nav")
				.children(&mut [
					// First button
					html!("button", {
						.class("pagination-btn")
						.text("«")
						.prop_signal("disabled", state.current_page.signal().map(|page| page <= 1))
						.event(clone!(state, on_page_change => move |_: events::Click| {
							if state.current_page.get() > 1 {
								on_page_change(1);
							}
						}))
					}),
					// Previous button
					html!("button", {
						.class("pagination-btn")
						.text("‹")
						.prop_signal("disabled", state.current_page.signal().map(|page| page <= 1))
						.event(clone!(state, on_page_change => move |_: events::Click| {
							let current = state.current_page.get();
							if current > 1 {
								on_page_change(current - 1);
							}
						}))
					}),
					// Page info
					html!("span", {
						.class("pagination-info")
						.text_signal(state.current_page.signal().map(clone!(state => move |current| {
							format!("Page {} of {}", current, state.total_pages.get())
						})))
					}),
					// Next button
					html!("button", {
						.class("pagination-btn")
						.text("›")
						.prop_signal("disabled", state.current_page.signal().map(clone!(state => move |page| {
							page >= state.total_pages.get()
						})))
						.event(clone!(state, on_page_change => move |_: events::Click| {
							let current = state.current_page.get();
							let total = state.total_pages.get();
							if current < total {
								on_page_change(current + 1);
							}
						}))
					}),
					// Last button
					html!("button", {
						.class("pagination-btn")
						.text("»")
						.prop_signal("disabled", state.current_page.signal().map(clone!(state => move |page| {
							page >= state.total_pages.get()
						})))
						.event(clone!(state, on_page_change => move |_: events::Click| {
							let total = state.total_pages.get();
							if state.current_page.get() < total {
								on_page_change(total);
							}
						}))
					}),
				])
			}),

			// Items per page selector
			html!("div", {
				.class("pagination-size")
				.children(&mut [
					html!("span", {
						.class("pagination-label")
						.text("Items per page:")
					}),
					html!("select" => web_sys::HtmlSelectElement, {
						.class("pagination-select")
						.prop_signal("value", state.page_size.signal().map(|size| size.to_string()))
						.children(&mut [
							html!("option", {
								.attr("value", "10")
								.text("10")
							}),
							html!("option", {
								.attr("value", "25")
								.text("25")
							}),
							html!("option", {
								.attr("value", "50")
								.text("50")
							}),
							html!("option", {
								.attr("value", "100")
								.text("100")
							}),
						])
						.event(clone!(state, on_page_size_change => move |event: events::Change| {
							let select: web_sys::HtmlSelectElement = event.target().unwrap().dyn_into().unwrap();
							if let Ok(size) = select.value().parse::<u64>() {
								state.page_size.set(size);
								if let Some(ref handler) = on_page_size_change {
									handler(size);
								}
							}
						}))
					}),
				])
			}),

			// Total items info
			html!("div", {
				.class("pagination-total")
				.text_signal(state.total_items.signal().map(|total| {
					format!("Total: {} items", total)
				}))
			}),
		])
	})
}
