use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{HtmlInputElement, window};

use reinhardt_pages::prelude::*;

const BENCHMARK_VERSION: &str = "baseline-version";

#[derive(Clone)]
struct Row {
	id: usize,
	label: String,
}

#[wasm_bindgen]
pub fn mount_benchmark_app() {
	let document = window()
		.and_then(|window| window.document())
		.expect("document is available");
	let root = Element::new(
		document
			.get_element_by_id("root")
			.expect("root element is available"),
	);
	benchmark_page(BENCHMARK_VERSION.to_string())
		.mount(&root)
		.expect("mount benchmark app");
}

fn benchmark_page(benchmark_version: String) -> Page {
	let (counter, set_counter) = use_shared_state(0usize);
	let (input_text, set_input_text) = use_shared_state(String::new());
	let (route, set_route) = use_shared_state(current_route());
	let (rows, set_rows) = use_shared_state(initial_rows());

	let increment_counter = Callback::new({
		let counter = counter.clone();
		let set_counter = set_counter.clone();
		move |_| {
			set_counter(counter.get() + 1);
		}
	});
	let update_input = Callback::new({
		let set_input_text = set_input_text.clone();
		move |event: Event| {
			let value = event
				.target()
				.and_then(|target| target.dyn_into::<HtmlInputElement>().ok())
				.map(|input| input.value())
				.unwrap_or_default();
			set_input_text(value);
		}
	});
	let route_home = route_callback(&set_route, "home", "/");
	let route_detail = route_callback(&set_route, "detail", "/detail");
	let route_form = route_callback(&set_route, "form", "/form");
	let append_row = Callback::new({
		let rows = rows.clone();
		let set_rows = set_rows.clone();
		move |_| {
			let mut next_rows = rows.get();
			let next_id = next_rows.len() + 1;
			next_rows.push(Row {
				id: next_id,
				label: format!("Row {next_id}"),
			});
			set_rows(next_rows);
		}
	});
	let reorder_rows = Callback::new({
		let rows = rows.clone();
		let set_rows = set_rows.clone();
		move |_| {
			let mut next_rows = rows.get();
			next_rows.reverse();
			set_rows(next_rows);
		}
	});
	let input_text_value = input_text.clone();
	let input_text_output = input_text.clone();
	let rows_count = rows.clone();
	let rows_first = rows.clone();
	let rows_list = rows.clone();

	page!(|benchmark_version: String,
		counter: SharedSignal<usize>,
		input_text_value: SharedSignal<String>,
		input_text_output: SharedSignal<String>,
		route: SharedSignal<String>,
		rows_count: SharedSignal<Vec<Row>>,
		rows_first: SharedSignal<Vec<Row>>,
		rows_list: SharedSignal<Vec<Row>>,
		increment_counter: Callback,
		update_input: Callback,
		route_home: Callback,
		route_detail: Callback,
		route_form: Callback,
		append_row: Callback,
		reorder_rows: Callback| {
		main {
			class: "bench-shell",
			data_benchmark_ready: "true",
			data_benchmark_hydrated: "true",
			h1 { "Reinhardt Pages Benchmark" }
			p {
				data_benchmark_value: "version",
				{ benchmark_version.clone() }
			}

			section {
				data_benchmark_scenario: "counter",
				button {
					data_benchmark_action: "counter-increment",
					@click: increment_counter,
					"Increment"
				}
				output {
					data_benchmark_value: "counter",
					{ format!("Counter: {}", counter.get()) }
				}
			}

			section {
				data_benchmark_scenario: "form-input",
				label {
					"Input"
					input {
						data_benchmark_action: "input",
						value: input_text_value.get(),
						@input: update_input,
					}
				}
				output {
					data_benchmark_value: "input",
					{ input_text_output.get() }
				}
			}

			section {
				data_benchmark_scenario: "router",
				nav {
					button {
						data_benchmark_action: "route-home",
						@click: route_home,
						"Home"
					}
					button {
						data_benchmark_action: "route-detail",
						@click: route_detail,
						"Detail"
					}
					button {
						data_benchmark_action: "route-form",
						@click: route_form,
						"Form"
					}
				}
				output {
					data_benchmark_value: "route",
					{ format!("Route: {}", route.get()) }
				}
			}

			section {
				data_benchmark_scenario: "keyed-list",
				button {
					data_benchmark_action: "list-append",
					@click: append_row,
					"Append row"
				}
				button {
					data_benchmark_action: "list-reorder",
					@click: reorder_rows,
					"Reorder rows"
				}
				output {
					data_benchmark_value: "list-count",
					{ format!("Rows: {}", rows_count.get().len()) }
				}
				output {
					data_benchmark_value: "list-first",
					{ rows_first.get().first().map(|row| format!("First: {}", row.label)).unwrap_or_else(|| "First: ".to_string()) }
				}
				ul {
					for row in rows_list.get().into_iter().take(25) @key(row.id.to_string()) {
						li {
							data_benchmark_row: row.id.to_string(),
							{ row.label.clone() }
						}
					}
				}
			}
		}
	})(
		benchmark_version,
		counter,
		input_text_value,
		input_text_output,
		route,
		rows_count,
		rows_first,
		rows_list,
		increment_counter,
		update_input,
		route_home,
		route_detail,
		route_form,
		append_row,
		reorder_rows,
	)
}

fn route_callback(set_route: &SharedSetState<String>, route: &str, path: &str) -> Callback {
	let set_route = set_route.clone();
	let route = route.to_string();
	let path = path.to_string();
	Callback::new(move |_| {
		if let Some(window) = window() {
			if let Ok(history) = window.history() {
				let _ = history.push_state_with_url(&JsValue::NULL, "", Some(&path));
			}
		}
		set_route(route.clone());
	})
}

fn current_route() -> String {
	let pathname = window()
		.and_then(|window| window.location().pathname().ok())
		.unwrap_or_default();
	match pathname.as_str() {
		"/detail" => "detail",
		"/form" => "form",
		_ => "home",
	}
	.to_string()
}

fn initial_rows() -> Vec<Row> {
	(1..=1000)
		.map(|id| Row {
			id,
			label: format!("Row {id}"),
		})
		.collect()
}
