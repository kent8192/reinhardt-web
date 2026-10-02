//! RAII browser fixtures with private fetch state and retained DOM queries.

use js_sys::{Function, Reflect};
use reinhardt_pages::component::{Page, PageExt, cleanup_reactive_nodes};
use reinhardt_pages::dom::Element;
use wasm_bindgen::{JsCast, JsValue};

pub(crate) fn flush() {
	reinhardt_pages::reactive::with_runtime(|runtime| runtime.flush_updates());
}

pub(crate) async fn wait_for(mut ready: impl FnMut() -> bool) {
	for _ in 0..200 {
		flush();
		if ready() {
			return;
		}
		gloo_timers::future::TimeoutFuture::new(0).await;
	}
	assert!(ready(), "browser state did not become ready");
}

pub(crate) struct BrowserRoot(pub(crate) web_sys::Element);

struct HydratedPage(Page);

impl reinhardt_pages::component::Component for HydratedPage {
	fn render(&self) -> Page {
		self.0.clone()
	}
	fn name() -> &'static str {
		"ClientFormFixture"
	}
}

struct HydrationState(web_sys::Element);

impl Drop for HydrationState {
	fn drop(&mut self) {
		self.0.remove();
	}
}

#[cfg(feature = "testing")]
pub(crate) struct RouterGuard {
	href: String,
	state: JsValue,
}

#[cfg(feature = "testing")]
impl RouterGuard {
	pub(crate) fn install(router: reinhardt_urls::routers::ClientRouter) -> Self {
		let window = web_sys::window().unwrap();
		let guard = Self {
			href: window.location().href().unwrap(),
			state: window.history().unwrap().state().unwrap(),
		};
		reinhardt_pages::app::__install_client_router_for_test(router);
		guard
	}
}

#[cfg(feature = "testing")]
impl Drop for RouterGuard {
	fn drop(&mut self) {
		reinhardt_pages::app::__clear_spa_router_for_test();
		web_sys::window()
			.unwrap()
			.history()
			.unwrap()
			.replace_state_with_url(&self.state, "", Some(&self.href))
			.unwrap();
	}
}

pub(crate) struct EventListenerGuard {
	target: web_sys::EventTarget,
	event: &'static str,
	callback: wasm_bindgen::closure::Closure<dyn Fn(web_sys::Event)>,
}
impl EventListenerGuard {
	pub(crate) fn new(
		target: &web_sys::EventTarget,
		event: &'static str,
		callback: impl Fn(web_sys::Event) + 'static,
	) -> Self {
		let callback = wasm_bindgen::closure::Closure::new(callback);
		target
			.add_event_listener_with_callback(event, callback.as_ref().unchecked_ref())
			.unwrap();
		Self {
			target: target.clone(),
			event,
			callback,
		}
	}
}
impl Drop for EventListenerGuard {
	fn drop(&mut self) {
		self.target
			.remove_event_listener_with_callback(self.event, self.callback.as_ref().unchecked_ref())
			.unwrap();
	}
}
impl BrowserRoot {
	pub(crate) fn new() -> Self {
		let document = web_sys::window().unwrap().document().unwrap();
		let root = Self(document.create_element("div").unwrap());
		document.body().unwrap().append_child(&root.0).unwrap();
		root
	}
	pub(crate) fn mount(page: &Page) -> Self {
		let root = Self::new();
		page.clone().mount(&Element::new(root.0.clone())).unwrap();
		flush();
		root
	}
	pub(crate) fn get<T: JsCast>(&self, selector: &str) -> T {
		self.0
			.query_selector(selector)
			.unwrap()
			.unwrap_or_else(|| panic!("missing {selector}"))
			.unchecked_into()
	}
	pub(crate) fn input(&self, name: &str, value: &str) -> web_sys::HtmlInputElement {
		let input: web_sys::HtmlInputElement = self.get(&format!("input[name='{name}']"));
		input.set_value(value);
		input
			.dispatch_event(&web_sys::Event::new("input").unwrap())
			.unwrap();
		flush();
		input
	}
	pub(crate) fn select(&self, name: &str, value: &str) {
		let select: web_sys::HtmlSelectElement = self.get(&format!("select[name='{name}']"));
		select.set_value(value);
		select
			.dispatch_event(&web_sys::Event::new("change").unwrap())
			.unwrap();
		flush();
	}
	pub(crate) fn submit(&self) {
		let form: web_sys::HtmlFormElement = self.get("form");
		let init = web_sys::EventInit::new();
		init.set_bubbles(true);
		init.set_cancelable(true);
		let event = web_sys::Event::new_with_event_init_dict("submit", &init).unwrap();
		assert!(
			!form.dispatch_event(&event).unwrap(),
			"generated submit must prevent navigation"
		);
		flush();
	}
	pub(crate) fn hydrate(&self, page: &Page) {
		let document = web_sys::window().unwrap().document().unwrap();
		let state = HydrationState(document.create_element("script").unwrap());
		state.0.set_id("ssr-state");
		state.0.set_attribute("type", "application/json").unwrap();
		state.0.set_text_content(Some("{}"));
		self.0.append_child(&state.0).unwrap();
		let form = Element::new(self.0.first_element_child().unwrap());
		reinhardt_pages::hydration::hydrate(&HydratedPage(page.clone()), &form).unwrap();
		flush();
	}
}
impl Drop for BrowserRoot {
	fn drop(&mut self) {
		self.0.remove();
		cleanup_reactive_nodes();
	}
}

pub(crate) struct FetchGuard {
	window: web_sys::Window,
	previous: JsValue,
	state: JsValue,
}
impl FetchGuard {
	pub(crate) fn install() -> Self {
		let window = web_sys::window().unwrap();
		let previous = Reflect::get(&window, &"fetch".into()).unwrap();
		let state = Function::new_no_args(r#"
            const requests = [];
            const pending = [];
            return {
                fetch(request) {
                    return request.clone().text().then(body => new Promise((resolve, reject) => {
                        requests.push({path: new URL(request.url).pathname, body: JSON.parse(body)});
                        pending.push({resolve, reject});
                    }));
                },
                requests() { return JSON.stringify(requests); },
                resolve(index, status, body) {
                    const item = pending[index];
                    pending[index] = null;
                    item.resolve(new Response(body, {status, headers: {'content-type': 'application/json'}}));
                },
                cancel() {
                    for (const item of pending) if (item) item.reject(new Error('Fixture disposed'));
                }
            };
        "#).call0(&JsValue::NULL).unwrap();
		Reflect::set(
			&window,
			&"fetch".into(),
			&Reflect::get(&state, &"fetch".into()).unwrap(),
		)
		.unwrap();
		Self {
			window,
			previous,
			state,
		}
	}
	fn method(&self, name: &str) -> Function {
		Reflect::get(&self.state, &name.into())
			.unwrap()
			.unchecked_into()
	}
	pub(crate) fn requests(&self) -> Vec<serde_json::Value> {
		let json = self
			.method("requests")
			.call0(&self.state)
			.unwrap()
			.as_string()
			.unwrap();
		serde_json::from_str(&json).unwrap()
	}
	pub(crate) fn resolve(&self, index: usize, status: u16, body: serde_json::Value) {
		self.method("resolve")
			.call3(
				&self.state,
				&JsValue::from_f64(index as f64),
				&JsValue::from_f64(f64::from(status)),
				&serde_json::to_string(&body).unwrap().into(),
			)
			.unwrap();
	}
}
impl Drop for FetchGuard {
	fn drop(&mut self) {
		Reflect::set(&self.window, &"fetch".into(), &self.previous).unwrap();
		self.method("cancel").call0(&self.state).unwrap();
	}
}
