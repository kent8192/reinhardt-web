//! Test: Complex watch block scenarios
//!
//! Validates advanced usage patterns combining multiple features.

use reinhardt_pages::Signal;
use reinhardt_pages::page;

fn main() {
	// Complex nested structure with multiple watches
	let _complex = page!(|loading: Signal<bool>, error: Signal<Option<String>>, items: Signal<Vec<String>>| {
		div {
			class: "container",
			h1 {
				"Dashboard"
			}
			watch {
				if error.get().is_some() {
					div {
						class: "alert alert-danger",
						{ error.get().unwrap_or_default() }
					}
				}
			}
			watch {
				if loading.get() {
					div {
						class: "spinner",
						"Loading..."
					}
				} else if items.get().is_empty() {
					p {
						"No items available"
					}
				} else {
					ul {
						class: "item-list",
						for item in items.get().iter() {
							li {
								class: "item",
								{ item.clone() }
							}
						}
					}
				}
			}
		}
	});

	// Watch with deeply nested conditions
	let _deep_nesting = page!(|a: Signal<bool>, b: Signal<bool>, c: Signal<bool>| {
		div {
			watch {
				if a.get() {
					div {
						if b.get() {
							span {
								if c.get() {
									"All true"
								} else {
									"A and B true"
								}
							}
						} else {
							span {
								"Only A true"
							}
						}
					}
				} else {
					span {
						"A is false"
					}
				}
			}
		}
	});
}
