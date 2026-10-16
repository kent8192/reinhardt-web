//! JavaScript Runtime for SSR
//!
//! This module provides a pure Rust JavaScript runtime using boa_engine,
//! enabling React/Preact SSR without external runtime dependencies.
//!
//! # Architecture
//!
//! The JavaScript runtime runs on the **host side** (not inside WASM plugins).
//! WASM plugins request SSR through host functions, and the host executes
//! JavaScript using this runtime.
//!
//! # Thread Safety
//!
//! `boa_engine::Context` is intentionally `!Send + !Sync` because it may rely
//! on thread-local state. To provide safe multi-threaded access, `TsRuntime`
//! uses a dedicated thread that owns the `Context` and communicates via
//! `std::sync::mpsc` channels. This makes `TsRuntime` naturally `Send + Sync`
//! without requiring `unsafe impl`.
//!
//! # Features
//!
//! - Pure Rust ECMAScript engine (no V8, no native dependencies)
//! - Preact bundled locally for SSR
//! - Thread-safe via dedicated runtime thread and channel communication
//!
//! # Example
//!
//! ```ignore
//! use reinhardt_dentdelion::wasm::TsRuntime;
//!
//! let runtime = TsRuntime::new()?;
//!
//! // Evaluate simple JavaScript
//! let result: String = runtime.eval("(1 + 2).toString()")?;
//! assert_eq!(result, "3");
//!
//! // Render a Preact component
//! let component_code = r#"
//!     function Component(props) {
//!         return h('div', null, 'Hello, ' + props.name);
//!     }
//! "#;
//! let html = runtime.render_component(component_code, r#"{"name": "World"}"#)?;
//! assert!(html.contains("Hello, World"));
//! ```

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread;
use std::time::{Duration, Instant};

use boa_engine::builtins::function::BoundFunction;
use boa_engine::object::FunctionObjectBuilder;
use boa_engine::property::Attribute;
use boa_engine::{Context, JsError, JsObject, JsValue, Script, Source};
use boa_engine::{JsNativeError, JsResult, NativeFunction, js_string};

/// Default maximum JavaScript execution time per evaluation (5 seconds).
const DEFAULT_EXECUTION_TIMEOUT: Duration = Duration::from_secs(5);

/// Default maximum JavaScript source code size in bytes (1 MiB).
const DEFAULT_MAX_SOURCE_SIZE: usize = 1024 * 1024;

/// Independent interpreter guard for loops reentered from native builtins.
const DEFAULT_LOOP_ITERATION_LIMIT: u64 = 1_000_000;

/// Resource limits for JavaScript execution.
///
/// Controls CPU and memory usage to prevent denial-of-service from
/// malicious or buggy JavaScript code.
#[derive(Debug, Clone)]
pub struct TsResourceLimits {
	/// Maximum execution time per evaluation.
	pub execution_timeout: Duration,
	/// Maximum source code size in bytes.
	pub max_source_size: usize,
	/// Maximum iterations of an individual loop, independently of elapsed time.
	///
	/// Boa native builtins can reenter synchronous JavaScript evaluation, where
	/// the cooperative deadline cannot run. This guard also bounds those loops.
	pub loop_iteration_limit: u64,
}

impl Default for TsResourceLimits {
	fn default() -> Self {
		Self {
			execution_timeout: DEFAULT_EXECUTION_TIMEOUT,
			max_source_size: DEFAULT_MAX_SOURCE_SIZE,
			loop_iteration_limit: DEFAULT_LOOP_ITERATION_LIMIT,
		}
	}
}

/// Preact core library (minified)
const PREACT_CORE: &str = include_str!("js/preact.min.js");

/// Preact render-to-string library (minified)
const PREACT_RENDER_TO_STRING: &str = include_str!("js/preact-render-to-string.min.js");

/// Command sent to the dedicated JavaScript runtime thread.
///
/// Each variant carries a response channel for returning the result
/// back to the calling thread.
enum RuntimeCommand {
	/// Evaluate JavaScript code and return the result as a string.
	Eval {
		code: String,
		response_tx: mpsc::Sender<Result<String, TsError>>,
	},
	/// Evaluate JavaScript code without expecting a return value.
	EvalVoid {
		code: String,
		response_tx: mpsc::Sender<Result<(), TsError>>,
	},
	/// Render a Preact component to HTML string.
	RenderComponent {
		component_code: String,
		props_json: String,
		response_tx: mpsc::Sender<Result<String, TsError>>,
	},
}

const COMMAND_QUEUED: u8 = 0;
const COMMAND_STARTED: u8 = 1;
const COMMAND_CANCELLED: u8 = 2;
const COMMAND_TIMED_OUT: u8 = 3;

/// A queued command and its worker-start acknowledgement.
struct RuntimeRequest {
	command: RuntimeCommand,
	started_tx: mpsc::Sender<()>,
	state: Arc<AtomicU8>,
	/// A host invocation may impose a shorter deadline including queue admission.
	deadline: Option<Instant>,
}

/// JavaScript runtime for SSR.
///
/// This runtime provides a sandboxed JavaScript execution environment
/// using boa_engine (pure Rust ECMAScript implementation).
///
/// # Thread Safety
///
/// `boa_engine::Context` is `!Send + !Sync` by design. Instead of using
/// `unsafe impl Send/Sync`, this struct uses a dedicated thread that owns
/// the `Context` and communicates via `std::sync::mpsc::SyncSender`.
/// `SyncSender<T>` is `Send + Sync` when `T: Send`, making `TsRuntime`
/// safely shareable across threads without `unsafe`.
pub struct TsRuntime {
	/// Channel to send commands to the dedicated runtime thread.
	///
	/// `SyncSender` is both `Send` and `Sync` (when `T: Send`),
	/// which makes `TsRuntime` naturally `Send + Sync`.
	command_tx: SyncSender<RuntimeRequest>,
	/// Whether Preact has been initialized (set once during construction)
	preact_initialized: bool,
	/// Resource limits for JavaScript execution
	limits: TsResourceLimits,
	/// Stop admitting work after a timed-out evaluation may still be running.
	poisoned: AtomicBool,
}

impl TsRuntime {
	/// Create a new JavaScript runtime with Preact pre-loaded and default resource limits.
	///
	/// This spawns a dedicated thread that owns the `boa_engine::Context`
	/// and initializes Preact libraries. The `Context` never leaves this
	/// thread, ensuring safe single-threaded access.
	///
	/// # Errors
	///
	/// Returns an error if:
	/// - Runtime thread creation fails
	/// - Preact initialization fails
	pub fn new() -> Result<Self, TsError> {
		Self::with_limits(TsResourceLimits::default())
	}

	/// Create a new JavaScript runtime with custom resource limits.
	///
	/// # Arguments
	///
	/// * `limits` - Resource limits controlling execution timeout and source size
	///
	/// # Errors
	///
	/// Returns an error if:
	/// - Runtime thread creation fails
	/// - Preact initialization fails
	pub fn with_limits(limits: TsResourceLimits) -> Result<Self, TsError> {
		// Bounded channel for command passing (backpressure at 16 pending commands)
		let (command_tx, command_rx) = mpsc::sync_channel::<RuntimeRequest>(16);

		// Oneshot channel for initialization result
		let (init_tx, init_rx) = mpsc::channel::<Result<(), TsError>>();

		// Spawn dedicated thread for boa_engine::Context.
		// Context is !Send + !Sync, so it must stay on this single thread.
		let execution_timeout = limits.execution_timeout;
		let loop_iteration_limit = limits.loop_iteration_limit;
		thread::spawn(move || {
			runtime_thread_main(command_rx, init_tx, execution_timeout, loop_iteration_limit);
		});

		// Wait for initialization result from the dedicated thread
		init_rx.recv().map_err(|_| {
			TsError::InitFailed("Runtime thread terminated during initialization".to_string())
		})??;

		Ok(Self {
			command_tx,
			preact_initialized: true,
			limits,
			poisoned: AtomicBool::new(false),
		})
	}

	/// Evaluate JavaScript code and return the result as a string.
	///
	/// # Arguments
	///
	/// * `code` - JavaScript code to evaluate
	///
	/// # Example
	///
	/// ```ignore
	/// let result = runtime.eval("(1 + 2).toString()")?;
	/// assert_eq!(result, "3");
	/// ```
	pub fn eval(&self, code: &str) -> Result<String, TsError> {
		self.eval_until(code, None)
	}

	pub(super) fn eval_until(
		&self,
		code: &str,
		deadline: Option<Instant>,
	) -> Result<String, TsError> {
		self.check_source_size(code)?;
		self.check_ready()?;

		self.execute(
			|response_tx| RuntimeCommand::Eval {
				code: code.to_string(),
				response_tx,
			},
			TsError::EvalFailed,
			deadline,
		)
	}

	/// Evaluate JavaScript code without expecting a return value.
	///
	/// Useful for executing side-effect code like defining functions.
	pub fn eval_void(&self, code: &str) -> Result<(), TsError> {
		self.check_source_size(code)?;
		self.check_ready()?;

		self.execute(
			|response_tx| RuntimeCommand::EvalVoid {
				code: code.to_string(),
				response_tx,
			},
			TsError::EvalFailed,
			None,
		)
	}

	/// Render a Preact/React component to HTML string.
	///
	/// # Arguments
	///
	/// * `component_code` - JavaScript code defining the component.
	///   Must define a function named `Component`.
	/// * `props_json` - JSON string representing component props
	///
	/// # Component Code Format
	///
	/// The component code should define a `Component` function:
	///
	/// ```javascript
	/// function Component(props) {
	///     return h('div', { class: 'container' },
	///         h('h1', null, props.title),
	///         h('p', null, props.content)
	///     );
	/// }
	/// ```
	///
	/// # Example
	///
	/// ```ignore
	/// let component = r#"
	///     function Component(props) {
	///         return h('div', null, 'Hello, ' + props.name);
	///     }
	/// "#;
	/// let html = runtime.render_component(component, r#"{"name": "World"}"#)?;
	/// assert!(html.contains("Hello, World"));
	/// ```
	pub fn render_component(
		&self,
		component_code: &str,
		props_json: &str,
	) -> Result<String, TsError> {
		self.render_component_until(component_code, props_json, None)
	}

	pub(super) fn render_component_until(
		&self,
		component_code: &str,
		props_json: &str,
		deadline: Option<Instant>,
	) -> Result<String, TsError> {
		if !self.preact_initialized {
			return Err(TsError::InitFailed("Preact not initialized".to_string()));
		}
		self.check_ready()?;

		// Check combined size of component code and props
		let total_size = component_code.len().saturating_add(props_json.len());
		if total_size > self.limits.max_source_size {
			return Err(TsError::SourceTooLarge {
				size: total_size,
				max: self.limits.max_source_size,
			});
		}

		self.execute(
			|response_tx| RuntimeCommand::RenderComponent {
				component_code: component_code.to_string(),
				props_json: props_json.to_string(),
				response_tx,
			},
			TsError::RenderFailed,
			deadline,
		)
	}

	/// Ordinary callers have separate admission/execution budgets. Host deadlines
	/// are absolute and constrain both waits without being reset at worker admission.
	fn execute<T>(
		&self,
		command: impl FnOnce(mpsc::Sender<Result<T, TsError>>) -> RuntimeCommand,
		unavailable: fn(String) -> TsError,
		deadline: Option<Instant>,
	) -> Result<T, TsError> {
		let remaining = || {
			deadline.map_or(self.limits.execution_timeout, |deadline| {
				self.limits
					.execution_timeout
					.min(deadline.saturating_duration_since(Instant::now()))
			})
		};
		if remaining().is_zero() {
			return Err(unavailable(
				"Host execution deadline expired before admission".into(),
			));
		}
		let (response_tx, response_rx) = mpsc::channel();
		let (started_tx, started_rx) = mpsc::channel();
		let state = Arc::new(AtomicU8::new(COMMAND_QUEUED));
		self.command_tx
			.try_send(RuntimeRequest {
				command: command(response_tx),
				started_tx,
				state: Arc::clone(&state),
				deadline,
			})
			.map_err(|_| unavailable("Runtime queue is unavailable or full".into()))?;

		match started_rx.recv_timeout(remaining()) {
			Ok(()) => {}
			Err(mpsc::RecvTimeoutError::Timeout) => {
				// The worker and caller race to admit or cancel this command.
				// Cancellation wins only while the command is still queued.
				if state
					.compare_exchange(
						COMMAND_QUEUED,
						COMMAND_CANCELLED,
						Ordering::AcqRel,
						Ordering::Acquire,
					)
					.is_ok()
				{
					return Err(unavailable("Runtime command timed out while queued".into()));
				}
			}
			Err(mpsc::RecvTimeoutError::Disconnected) => {
				return Err(unavailable(
					"Runtime thread terminated before evaluation".into(),
				));
			}
		}

		let timeout = remaining();
		let result = response_rx
			.recv_timeout(timeout)
			.map_err(|error| match error {
				mpsc::RecvTimeoutError::Timeout => {
					state.store(COMMAND_TIMED_OUT, Ordering::Release);
					self.poisoned.store(true, Ordering::Release);
					TsError::ExecutionTimeout { timeout }
				}
				mpsc::RecvTimeoutError::Disconnected => {
					unavailable("Runtime thread terminated during evaluation".into())
				}
			})?;
		if matches!(result, Err(TsError::ExecutionTimeout { .. })) {
			self.poisoned.store(true, Ordering::Release);
		}
		result
	}

	/// Check if Preact is initialized and ready for SSR.
	pub fn is_preact_ready(&self) -> bool {
		self.preact_initialized
	}

	/// Get the current resource limits.
	pub fn limits(&self) -> &TsResourceLimits {
		&self.limits
	}

	/// Validate source code size against the configured limit.
	fn check_source_size(&self, code: &str) -> Result<(), TsError> {
		if code.len() > self.limits.max_source_size {
			return Err(TsError::SourceTooLarge {
				size: code.len(),
				max: self.limits.max_source_size,
			});
		}
		Ok(())
	}

	fn check_ready(&self) -> Result<(), TsError> {
		if self.poisoned.load(Ordering::Acquire) {
			Err(TsError::ExecutionTimeout {
				timeout: self.limits.execution_timeout,
			})
		} else {
			Ok(())
		}
	}
}

/// Main function for the dedicated JavaScript runtime thread.
///
/// This thread owns the `boa_engine::Context` (which is `!Send + !Sync`)
/// and processes commands received via channel. The context never leaves
/// this thread, ensuring safe single-threaded access without `unsafe`.
fn runtime_thread_main(
	command_rx: Receiver<RuntimeRequest>,
	init_tx: mpsc::Sender<Result<(), TsError>>,
	execution_timeout: Duration,
	loop_iteration_limit: u64,
) {
	let mut context = Context::default();

	// Keep the converter and its intrinsic captures outside the guest's globals.
	let stringifier = match init_preact(&mut context).and_then(|_| init_stringifier(&mut context)) {
		Ok(stringifier) => {
			if init_tx.send(Ok(())).is_err() {
				return;
			}
			stringifier
		}
		Err(error) => {
			let _ = init_tx.send(Err(error));
			return;
		}
	};
	context
		.runtime_limits_mut()
		.set_loop_iteration_limit(loop_iteration_limit);
	// Process commands until the channel is closed (all senders dropped)
	while let Ok(request) = command_rx.recv() {
		if request
			.state
			.compare_exchange(
				COMMAND_QUEUED,
				COMMAND_STARTED,
				Ordering::AcqRel,
				Ordering::Acquire,
			)
			.is_err()
		{
			continue;
		}
		let _ = request.started_tx.send(());
		let timed_out = match request.command {
			RuntimeCommand::Eval { code, response_tx } => {
				let result = eval_code(
					&code,
					&mut context,
					&stringifier,
					execution_timeout,
					request.deadline,
				);
				let timed_out = matches!(result, Err(TsError::ExecutionTimeout { .. }));
				let _ = response_tx.send(result);
				timed_out
			}
			RuntimeCommand::EvalVoid { code, response_tx } => {
				let result =
					eval_void_code(&code, &mut context, execution_timeout, request.deadline);
				let timed_out = matches!(result, Err(TsError::ExecutionTimeout { .. }));
				let _ = response_tx.send(result);
				timed_out
			}
			RuntimeCommand::RenderComponent {
				component_code,
				props_json,
				response_tx,
			} => {
				let result = render_component_code(
					&component_code,
					&props_json,
					&mut context,
					&stringifier,
					execution_timeout,
					request.deadline,
				);
				let timed_out = matches!(result, Err(TsError::ExecutionTimeout { .. }));
				let _ = response_tx.send(result);
				timed_out
			}
		};
		if timed_out || request.state.load(Ordering::Acquire) == COMMAND_TIMED_OUT {
			// Cancelling Boa's evaluation leaves VM frames suspended. Discard the
			// entire context instead of admitting another command into that state.
			break;
		}
	}
}

/// Initialize Preact libraries in the JavaScript context.
fn init_preact(context: &mut Context) -> Result<(), TsError> {
	// Load Preact core
	context.eval(Source::from_bytes(PREACT_CORE)).map_err(|e| {
		TsError::InitFailed(format!(
			"Failed to load Preact core: {}",
			js_error_to_string(&e, context)
		))
	})?;

	// Load Preact render-to-string
	context
		.eval(Source::from_bytes(PREACT_RENDER_TO_STRING))
		.map_err(|e| {
			TsError::InitFailed(format!(
				"Failed to load Preact render-to-string: {}",
				js_error_to_string(&e, context)
			))
		})?;

	// Create global bindings for convenience
	// This makes `h`, `Fragment`, and `renderToString` available directly in the global scope
	const GLOBAL_BINDINGS: &str = r#"
		var h = preact.h;
		var Fragment = preact.Fragment;
		var createElement = preact.createElement;
		var renderToString = preactRenderToString.renderToString;
		var renderToStaticMarkup = preactRenderToString.renderToStaticMarkup;
	"#;
	context
		.eval(Source::from_bytes(GLOBAL_BINDINGS))
		.map_err(|e| {
			TsError::InitFailed(format!(
				"Failed to create global bindings: {}",
				js_error_to_string(&e, context)
			))
		})?;

	Ok(())
}

// A non-enumerable host bridge is overwritten only after guest evaluation.
const STRINGIFY_BRIDGE: &str = "__reinhardt_stringify_result";

/// Create a bound method whose guest bytecode runs in the outer async VM.
/// Function.prototype.call and String(object) would reenter the synchronous VM.
/// Workaround evidence: docs/boa-cooperative-deadline.md. Replace this converter
/// and its bridge when Boa provides cooperative ToString covering native reentry.
/// Ideal implementation (unavailable API pseudocode): await value.to_string_async(context)
/// under the evaluation's original deadline, without a guest-global binding.
fn bind_conversion_method(
	_: &JsValue,
	args: &[JsValue],
	context: &mut Context,
) -> JsResult<JsValue> {
	let method = args
		.first()
		.and_then(JsValue::as_callable)
		.ok_or_else(|| JsNativeError::typ().with_message("Conversion method is not callable"))?;
	BoundFunction::create(
		method,
		args.get(1).cloned().unwrap_or_default(),
		Vec::new(),
		context,
	)
	.map(JsValue::from)
}

fn init_stringifier(context: &mut Context) -> Result<JsObject, TsError> {
	let binder = FunctionObjectBuilder::new(
		context.realm(),
		NativeFunction::from_fn_ptr(bind_conversion_method),
	)
	.build();
	context
		.register_global_property(js_string!(STRINGIFY_BRIDGE), binder, Attribute::WRITABLE)
		.map_err(|error| TsError::InitFailed(error.to_string()))?;
	let result = context.eval(Source::from_bytes(
		r#"
		(function(bind, primitiveSymbol, TypeError) {
			function primitive(value) {
				return value === null || (typeof value !== 'object' && typeof value !== 'function');
			}
			return function(value) {
				if (primitive(value)) return '' + value;
				const exotic = value[primitiveSymbol];
				if (exotic !== undefined && exotic !== null) {
					if (typeof exotic !== 'function') throw new TypeError('Invalid @@toPrimitive');
					const result = bind(exotic, value)('string');
					if (primitive(result)) return '' + result;
					throw new TypeError('Cannot convert object to primitive value');
				}
				for (let index = 0; index < 2; index++) {
					const method = value[index === 0 ? 'toString' : 'valueOf'];
					if (typeof method === 'function') {
						const result = bind(method, value)();
						if (primitive(result)) return '' + result;
					}
				}
				throw new TypeError('Cannot convert object to primitive value');
			};
		})(__reinhardt_stringify_result, Symbol.toPrimitive, TypeError)
	"#,
	));
	context
		.register_global_property(
			js_string!(STRINGIFY_BRIDGE),
			JsValue::undefined(),
			Attribute::WRITABLE,
		)
		.map_err(|error| TsError::InitFailed(error.to_string()))?;
	result
		.map_err(|error| TsError::InitFailed(error.to_string()))?
		.as_callable()
		.ok_or_else(|| TsError::InitFailed("Missing result converter".into()))
}

/// Clear the result binding on every exit; a cancelled VM context is discarded.
struct StringifyBinding<'a>(&'a mut Context);
impl Drop for StringifyBinding<'_> {
	fn drop(&mut self) {
		// This ordinary global property cannot execute guest accessors. If the
		// guest freezes it, later conversion fails closed; Context owns its value.
		let _ = self.0.register_global_property(
			js_string!(STRINGIFY_BRIDGE),
			JsValue::undefined(),
			Attribute::WRITABLE,
		);
	}
}

/// Evaluate code and convert its completion value within one elapsed-time budget.
fn eval_code(
	code: &str,
	context: &mut Context,
	stringifier: &JsObject,
	timeout: Duration,
	deadline: Option<Instant>,
) -> Result<String, TsError> {
	let started = Instant::now();
	let timeout = timeout_until(timeout, deadline, started);
	let result = eval_until_deadline(code, context, timeout, started, false)?;
	js_value_to_string(&result, context, stringifier, timeout, started)
}

/// Evaluate JavaScript code without expecting a return value.
fn eval_void_code(
	code: &str,
	context: &mut Context,
	timeout: Duration,
	deadline: Option<Instant>,
) -> Result<(), TsError> {
	let started = Instant::now();
	eval_until_deadline(
		code,
		context,
		timeout_until(timeout, deadline, started),
		started,
		false,
	)?;
	Ok(())
}

/// Render a Preact component to HTML using the JavaScript context.
fn render_component_code(
	component_code: &str,
	props_json: &str,
	context: &mut Context,
	stringifier: &JsObject,
	timeout: Duration,
	deadline: Option<Instant>,
) -> Result<String, TsError> {
	let started = Instant::now();
	let timeout = timeout_until(timeout, deadline, started);
	let render_script = format!(
		r#"
		(function() {{
			{component_code}
			var props = {props_json};
			return renderToString(h(Component, props));
		}})()
		"#
	);

	let result = eval_until_deadline(&render_script, context, timeout, started, true)?;

	js_value_to_string(&result, context, stringifier, timeout, started)
}

fn timeout_until(timeout: Duration, deadline: Option<Instant>, started: Instant) -> Duration {
	// Anchor both values to the same Instant: descheduling cannot reset the host budget.
	deadline.map_or(timeout, |deadline| {
		timeout.min(deadline.saturating_duration_since(started))
	})
}

/// Poll Boa cooperatively so the elapsed-time deadline stops the interpreter.
/// The VM budget controls yielding frequency, not the number of loop iterations.
#[cfg(test)]
fn eval_with_deadline(
	code: &str,
	context: &mut Context,
	timeout: Duration,
	render: bool,
) -> Result<JsValue, TsError> {
	eval_until_deadline(code, context, timeout, Instant::now(), render)
}

fn eval_until_deadline(
	code: &str,
	context: &mut Context,
	timeout: Duration,
	started: Instant,
	render: bool,
) -> Result<JsValue, TsError> {
	use std::future::Future;
	use std::task::{Context as TaskContext, Poll, Waker};

	let map_error = |error: JsError, context: &mut Context| {
		if error
			.as_native()
			.is_some_and(|native| native.is_runtime_limit())
		{
			TsError::RuntimeLimitExceeded {
				message: error.to_string(),
			}
		} else if render {
			TsError::RenderFailed(js_error_to_string(&error, context))
		} else {
			TsError::EvalFailed(js_error_to_string(&error, context))
		}
	};
	let script = Script::parse(Source::from_bytes(code), None, context)
		.map_err(|error| map_error(error, context))?;
	let outcome = {
		let mut evaluation = std::pin::pin!(script.evaluate_async_with_budget(context, 256));
		let mut task_context = TaskContext::from_waker(Waker::noop());
		loop {
			if started.elapsed() >= timeout {
				break None;
			}
			match evaluation.as_mut().poll(&mut task_context) {
				Poll::Ready(result) => break Some(result),
				Poll::Pending => {}
			}
		}
	};
	if started.elapsed() >= timeout {
		return Err(TsError::ExecutionTimeout { timeout });
	}
	outcome
		.ok_or(TsError::ExecutionTimeout { timeout })?
		.map_err(|error| map_error(error, context))
}

/// Invoke the captured converter through cooperative VM execution, not native ToString.
fn js_value_to_string(
	value: &JsValue,
	context: &mut Context,
	stringifier: &JsObject,
	timeout: Duration,
	started: Instant,
) -> Result<String, TsError> {
	let bound = BoundFunction::create(
		stringifier.clone(),
		JsValue::undefined(),
		vec![value.clone()],
		context,
	)
	.map_err(|error| TsError::EvalFailed(error.to_string()))?;
	context
		.register_global_property(js_string!(STRINGIFY_BRIDGE), bound, Attribute::WRITABLE)
		.map_err(|error| TsError::EvalFailed(error.to_string()))?;
	let binding = StringifyBinding(context);
	let result = eval_until_deadline(
		"__reinhardt_stringify_result()",
		binding.0,
		timeout,
		started,
		false,
	)?;
	result
		.as_string()
		.map(|value| value.to_std_string_escaped())
		.ok_or_else(|| TsError::EvalFailed("Result converter did not return a string".into()))
}

/// Convert `JsError` to String
fn js_error_to_string(error: &JsError, _context: &mut Context) -> String {
	// Display reads the error representation without invoking user toString
	// hooks, which must never execute while formatting an evaluation failure.
	error.to_string()
}

/// Shared JavaScript runtime instance type.
pub type SharedTsRuntime = Arc<TsRuntime>;

/// JavaScript runtime errors.
///
/// This enum is marked `#[non_exhaustive]` to allow adding new error variants
/// in future minor versions without breaking downstream code. Match arms
/// should include a wildcard pattern.
#[derive(Debug, Clone, thiserror::Error)]
#[non_exhaustive]
pub enum TsError {
	/// The interpreter exhausted an operation budget independently of elapsed time.
	#[error("JavaScript interpreter resource limit exceeded: {message}")]
	RuntimeLimitExceeded {
		/// Interpreter diagnostic identifying the exhausted resource budget.
		message: String,
	},
	/// JavaScript runtime initialization failed
	#[error("JavaScript runtime initialization failed: {0}")]
	InitFailed(String),

	/// JavaScript evaluation failed
	#[error("JavaScript evaluation failed: {0}")]
	EvalFailed(String),

	/// Props serialization failed
	#[error("Props serialization failed: {0}")]
	PropsSerialization(String),

	/// Component rendering failed
	#[error("Component rendering failed: {0}")]
	RenderFailed(String),

	/// JavaScript execution exceeded the configured timeout
	#[error("JavaScript execution timed out after {timeout:?}")]
	ExecutionTimeout {
		/// The timeout duration that was exceeded
		timeout: Duration,
	},

	/// JavaScript source code exceeded the configured size limit
	#[error("JavaScript source size ({size} bytes) exceeds limit ({max} bytes)")]
	SourceTooLarge {
		/// Actual source size in bytes
		size: usize,
		/// Maximum allowed size in bytes
		max: usize,
	},
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_ts_runtime_creation() {
		// Arrange & Act
		let runtime = TsRuntime::new();

		// Assert
		assert!(runtime.is_ok());
		assert!(runtime.unwrap().is_preact_ready());
	}

	#[rstest]
	fn test_eval_simple_expression() {
		// Arrange
		let runtime = TsRuntime::new().unwrap();

		// Act
		let result = runtime.eval("(1 + 2).toString()").unwrap();

		// Assert
		assert_eq!(result, "3");
	}

	#[rstest]
	fn test_eval_string_expression() {
		// Arrange
		let runtime = TsRuntime::new().unwrap();

		// Act
		let result = runtime.eval("'Hello, ' + 'World'").unwrap();

		// Assert
		assert_eq!(result, "Hello, World");
	}

	#[rstest]
	fn test_eval_void() {
		// Arrange
		let runtime = TsRuntime::new().unwrap();

		// Act & Assert
		runtime.eval_void("var x = 42;").unwrap();
	}

	#[rstest]
	fn test_render_simple_component() {
		// Arrange
		let runtime = TsRuntime::new().unwrap();
		let component_code = r#"
			function Component(props) {
				return h('div', null, 'Hello, ' + props.name);
			}
		"#;
		let props = r#"{"name": "World"}"#;

		// Act
		let result = runtime.render_component(component_code, props);

		// Assert
		assert!(result.is_ok());
		let html = result.unwrap();
		assert!(html.contains("Hello, World"));
		assert!(html.contains("<div>"));
	}

	#[rstest]
	fn test_render_nested_component() {
		// Arrange
		let runtime = TsRuntime::new().unwrap();
		let component_code = r#"
			function Child(props) {
				return h('span', null, props.text);
			}
			function Component(props) {
				return h('div', { class: 'container' },
					h(Child, { text: props.message })
				);
			}
		"#;
		let props = r#"{"message": "Nested!"}"#;

		// Act
		let result = runtime.render_component(component_code, props);

		// Assert
		assert!(result.is_ok());
		let html = result.unwrap();
		assert!(html.contains("container"));
		assert!(html.contains("Nested!"));
	}

	#[rstest]
	fn test_render_with_attributes() {
		// Arrange
		let runtime = TsRuntime::new().unwrap();
		let component_code = r#"
			function Component(props) {
				return h('a', { href: props.url, class: 'link' }, props.label);
			}
		"#;
		let props = r#"{"url": "https://example.com", "label": "Click me"}"#;

		// Act
		let result = runtime.render_component(component_code, props);

		// Assert
		assert!(result.is_ok());
		let html = result.unwrap();
		assert!(html.contains("href="));
		assert!(html.contains("Click me"));
	}

	/// Verify `TsRuntime` is `Send + Sync` at compile time.
	///
	/// This is the core safety property: because the `boa_engine::Context`
	/// is confined to a dedicated thread and `TsRuntime` only holds a
	/// `SyncSender` (which is `Send + Sync`), the struct is naturally
	/// thread-safe without `unsafe impl`.
	#[rstest]
	fn test_ts_runtime_is_send_and_sync() {
		fn assert_send<T: Send>() {}
		fn assert_sync<T: Sync>() {}
		assert_send::<TsRuntime>();
		assert_sync::<TsRuntime>();
	}

	/// Verify `SharedTsRuntime` (`Arc<TsRuntime>`) can be shared across threads.
	#[rstest]
	fn test_shared_ts_runtime_across_threads() {
		// Arrange
		let runtime = Arc::new(TsRuntime::new().unwrap());
		let runtime_clone = Arc::clone(&runtime);

		// Act: use from another thread
		let handle = std::thread::spawn(move || runtime_clone.eval("(2 + 3).toString()").unwrap());

		// Assert
		let result = handle.join().unwrap();
		assert_eq!(result, "5");
	}

	// ==========================================================================
	// Resource Limits Tests (#690)
	// ==========================================================================

	#[rstest]
	fn test_source_size_limit_rejects_oversized_code() {
		// Arrange
		let limits = TsResourceLimits {
			max_source_size: 64,
			..TsResourceLimits::default()
		};
		let runtime = TsRuntime::with_limits(limits).unwrap();
		let oversized_code = "a".repeat(128);

		// Act
		let result = runtime.eval(&oversized_code);

		// Assert
		assert!(matches!(
			result,
			Err(TsError::SourceTooLarge { size: 128, max: 64 })
		));
	}

	#[rstest]
	fn test_source_size_limit_allows_within_limit() {
		// Arrange
		let limits = TsResourceLimits {
			max_source_size: 256,
			..TsResourceLimits::default()
		};
		let runtime = TsRuntime::with_limits(limits).unwrap();

		// Act
		let result = runtime.eval("(1 + 1).toString()");

		// Assert
		assert!(result.is_ok());
		assert_eq!(result.unwrap(), "2");
	}

	#[rstest]
	fn test_eval_void_source_size_limit() {
		// Arrange
		let limits = TsResourceLimits {
			max_source_size: 32,
			..TsResourceLimits::default()
		};
		let runtime = TsRuntime::with_limits(limits).unwrap();
		let oversized_code = "var x = ".to_string() + &"0".repeat(64);

		// Act
		let result = runtime.eval_void(&oversized_code);

		// Assert
		assert!(matches!(result, Err(TsError::SourceTooLarge { .. })));
	}

	#[rstest]
	fn test_render_component_source_size_limit() {
		// Arrange
		let limits = TsResourceLimits {
			max_source_size: 64,
			..TsResourceLimits::default()
		};
		let runtime = TsRuntime::with_limits(limits).unwrap();
		let oversized_component = "a".repeat(128);

		// Act
		let result = runtime.render_component(&oversized_component, "{}");

		// Assert
		assert!(matches!(result, Err(TsError::SourceTooLarge { .. })));
	}

	#[rstest]
	#[case("while (true) {}")]
	#[case("String({ toString() { while (true) {} } })")]
	#[case("[1].map(function () { while (true) {} })")]
	#[case("({ toString() { while (true) {} } })")]
	fn interpreter_budget_is_distinct_from_execution_timeout(#[case] code: &str) {
		let runtime = TsRuntime::with_limits(TsResourceLimits {
			loop_iteration_limit: 1000,
			execution_timeout: Duration::from_secs(2),
			..TsResourceLimits::default()
		})
		.unwrap();

		let result = runtime.eval(code);

		assert!(
			matches!(result, Err(TsError::RuntimeLimitExceeded { .. })),
			"unexpected result: {result:?}"
		);
	}

	#[rstest]
	fn thrown_object_error_does_not_execute_its_conversion_hook() {
		let runtime = TsRuntime::with_limits(TsResourceLimits {
			loop_iteration_limit: 1000,
			..TsResourceLimits::default()
		})
		.unwrap();

		let result = runtime.eval("throw { toString() { while (true) {} } }");

		assert!(matches!(result, Err(TsError::EvalFailed(_))));
	}

	#[rstest]
	fn queued_evaluations_each_receive_a_full_execution_budget() {
		// Arrange: the total work exceeds one budget, while each evaluation fits.
		let runtime = TsRuntime::with_limits(TsResourceLimits {
			execution_timeout: Duration::from_millis(500),
			..TsResourceLimits::default()
		})
		.unwrap();
		let barrier = std::sync::Barrier::new(3);
		let code = "(() => { const end = Date.now() + 300; while (Date.now() < end) {} return 'ready'; })()";

		// Act: both requests enter the same worker queue concurrently.
		thread::scope(|scope| {
			let handles: Vec<_> = (0..2)
				.map(|_| {
					scope.spawn(|| {
						barrier.wait();
						runtime.eval(code)
					})
				})
				.collect();
			barrier.wait();

			// Assert: queue time does not consume the evaluation's budget.
			for handle in handles {
				assert_eq!(handle.join().unwrap().unwrap(), "ready");
			}
		});
		assert_eq!(runtime.eval("'still-ready'").unwrap(), "still-ready");
	}

	#[rstest]
	fn cancelled_queued_commands_do_not_execute_or_poison_the_runtime() {
		// Arrange: two admitted evaluations keep the worker busy beyond a queue budget.
		let runtime = TsRuntime::with_limits(TsResourceLimits {
			execution_timeout: Duration::from_millis(400),
			..TsResourceLimits::default()
		})
		.unwrap();
		let code = "(() => { const end = Date.now() + 250; while (Date.now() < end) {} return 'ready'; })()";
		let mut responses = Vec::new();
		for _ in 0..2 {
			let (response_tx, response_rx) = mpsc::channel();
			let (started_tx, _started_rx) = mpsc::channel();
			runtime
				.command_tx
				.try_send(RuntimeRequest {
					command: RuntimeCommand::Eval {
						code: code.into(),
						response_tx,
					},
					started_tx,
					state: Arc::new(AtomicU8::new(COMMAND_QUEUED)),
					deadline: None,
				})
				.unwrap();
			responses.push(response_rx);
		}

		// Act: cancel a command that has not started.
		let result = runtime.eval("globalThis.cancelledCommandRan = true");

		// Assert: the runtime stays healthy and the cancelled side effect never runs.
		assert!(matches!(result, Err(TsError::EvalFailed(message)) if message.contains("queued")));
		for response in responses {
			assert_eq!(
				response
					.recv_timeout(Duration::from_secs(2))
					.unwrap()
					.unwrap(),
				"ready"
			);
		}
		assert_eq!(
			runtime
				.eval("typeof globalThis.cancelledCommandRan")
				.unwrap(),
			"undefined"
		);
	}

	#[rstest]
	#[case(
		"(() => { const end = Date.now() + 500; while (Date.now() < end) {} return 'late'; })()"
	)]
	#[case(
		"({ toString() { const end = Date.now() + 500; while (Date.now() < end) {} return 'late'; } })"
	)]
	fn host_deadline_reaches_the_worker_and_result_conversion(#[case] code: &str) {
		// Arrange: bypass the caller wait so it cannot conceal continued VM work.
		let runtime = TsRuntime::with_limits(TsResourceLimits {
			execution_timeout: Duration::from_secs(2),
			loop_iteration_limit: u64::MAX,
			..TsResourceLimits::default()
		})
		.unwrap();
		let (response_tx, response_rx) = mpsc::channel();
		let (started_tx, _started_rx) = mpsc::channel();
		let started = Instant::now();
		let budget = Duration::from_millis(100);

		// Act: the worker receives an absolute host deadline shorter than its own limit.
		runtime
			.command_tx
			.try_send(RuntimeRequest {
				command: RuntimeCommand::Eval {
					code: code.into(),
					response_tx,
				},
				started_tx,
				state: Arc::new(AtomicU8::new(COMMAND_QUEUED)),
				deadline: Some(started + budget),
			})
			.unwrap();
		let result = response_rx.recv_timeout(Duration::from_secs(1)).unwrap();

		// Assert: execution and conversion stop in the worker under the remaining budget.
		assert!(matches!(result, Err(TsError::ExecutionTimeout { timeout }) if timeout <= budget));
		assert!(started.elapsed() < Duration::from_millis(500));
	}

	#[rstest]
	fn expired_host_deadlines_do_not_admit_queued_or_delayed_commands() {
		// Arrange: an admitted public evaluation keeps the dedicated worker occupied.
		let runtime = TsRuntime::new().unwrap();
		let (response_tx, response_rx) = mpsc::channel();
		let (started_tx, started_rx) = mpsc::channel();
		runtime.command_tx.try_send(RuntimeRequest {
			command: RuntimeCommand::Eval {
				code: "(() => { const end = Date.now() + 200; while (Date.now() < end) {} return 'ready'; })()".into(),
				response_tx,
			}, started_tx, state: Arc::new(AtomicU8::new(COMMAND_QUEUED)), deadline: None,
		}).unwrap();
		started_rx.recv_timeout(Duration::from_secs(1)).unwrap();

		// Act: queue admission cannot restart a host deadline; delayed blocking work is rejected.
		let queued = runtime.eval_until(
			"globalThis.queuedHostRan = true",
			Some(Instant::now() + Duration::from_millis(30)),
		);
		let expired = runtime.eval_until("globalThis.delayedHostRan = true", Some(Instant::now()));

		// Assert: cancelled queued work has no effects and does not retire the healthy context.
		assert!(matches!(queued, Err(TsError::EvalFailed(message)) if message.contains("queued")));
		assert!(
			matches!(expired, Err(TsError::EvalFailed(message)) if message.contains("before admission"))
		);
		assert_eq!(
			response_rx
				.recv_timeout(Duration::from_secs(1))
				.unwrap()
				.unwrap(),
			"ready"
		);
		assert_eq!(
			runtime
				.eval("typeof queuedHostRan + ':' + typeof delayedHostRan")
				.unwrap(),
			"undefined:undefined"
		);
	}

	#[rstest]
	#[case(
		"({ toString() { const until = Date.now() + 1000; while (Date.now() < until) {} return 'late'; } })"
	)]
	#[case(
		"({ [Symbol.toPrimitive]() { const until = Date.now() + 1000; while (Date.now() < until) {} return 'late'; } })"
	)]
	fn result_conversion_obeys_the_same_cooperative_deadline(#[case] code: &str) {
		// Arrange: isolate the worker path so caller recv_timeout cannot hide ongoing conversion.
		let mut context = Context::default();
		let stringifier = init_stringifier(&mut context).unwrap();
		let timeout = Duration::from_millis(30);
		let started = Instant::now();

		// Act: the object has a finite but expensive guest conversion hook.
		let result = eval_code(code, &mut context, &stringifier, timeout, None);

		// Assert: the interpreter itself stops instead of continuing for a second.
		assert!(
			matches!(result, Err(TsError::ExecutionTimeout { timeout: limit }) if limit == timeout)
		);
		assert!(started.elapsed() < Duration::from_millis(500));
	}

	#[rstest]
	#[case("({ get toString() { while (true) {} } })")]
	#[case("({ toString() { return String({ toString() { while (true) {} } }); } })")]
	fn conversion_native_reentry_retains_the_independent_iteration_guard(#[case] code: &str) {
		// Arrange: use the worker directly so a caller timeout cannot hide continued execution.
		let mut context = Context::default();
		let stringifier = init_stringifier(&mut context).unwrap();
		context.runtime_limits_mut().set_loop_iteration_limit(1000);
		let started = Instant::now();

		// Act: getters and nested native String calls can reenter the synchronous VM.
		let result = eval_code(
			code,
			&mut context,
			&stringifier,
			Duration::from_secs(2),
			None,
		);

		// Assert: the independent guard stops the interpreter, rather than just the caller.
		assert!(matches!(result, Err(TsError::RuntimeLimitExceeded { .. })));
		assert!(started.elapsed() < Duration::from_millis(500));
	}

	#[rstest]
	#[case("({ value: 42, toString() { return this.value; } })", "42")]
	#[case(
		"({ value: 42, [Symbol.toPrimitive](hint) { return hint + this.value; } })",
		"string42"
	)]
	#[case("({ toString: null, valueOf() { return 7; } })", "7")]
	#[case("({ toString() { return {}; }, valueOf() { return 7; } })", "7")]
	#[case(
		"var reads = 0; ({ value: 42, get toString() { reads++; return function() { return this.value + ':' + reads; }; } })",
		"42:1"
	)]
	#[case("[1, 2, 3]", "1,2,3")]
	#[case("null", "null")]
	#[case("undefined", "undefined")]
	fn cooperative_conversion_preserves_string_hint_semantics(
		#[case] code: &str,
		#[case] expected: &str,
	) {
		let runtime = TsRuntime::new().unwrap();
		assert_eq!(runtime.eval(code).unwrap(), expected);
	}

	#[rstest]
	fn test_large_finite_loop_completes_within_execution_timeout() {
		let runtime = TsRuntime::new().unwrap();

		let result: String = runtime
			.eval("var n = 0; for (var i = 0; i < 150000; i++) { n++; } n.toString()")
			.unwrap();

		assert_eq!(result, "150000");
	}

	#[rstest]
	fn test_deadline_stops_the_interpreter_without_a_iteration_budget() {
		let mut context = Context::default();
		let timeout = Duration::from_millis(20);
		let started = Instant::now();

		let result = eval_with_deadline("while (true) {}", &mut context, timeout, false);

		assert!(
			matches!(result, Err(TsError::ExecutionTimeout { timeout: elapsed }) if elapsed == timeout)
		);
		assert!(started.elapsed() < Duration::from_secs(1));
	}

	#[rstest]
	fn test_execution_timeout_rejects_long_running_code() {
		// Arrange: use a very short timeout
		let limits = TsResourceLimits {
			execution_timeout: Duration::from_millis(100),
			max_source_size: DEFAULT_MAX_SOURCE_SIZE,
			loop_iteration_limit: u64::MAX,
		};
		let runtime = TsRuntime::with_limits(limits).unwrap();
		// Intentionally long-running JavaScript (busy loop)
		let infinite_loop = "var i = 0; while(true) { i++; }";

		// Act
		let result = runtime.eval(infinite_loop);

		// Assert
		assert!(matches!(result, Err(TsError::ExecutionTimeout { .. })));
	}

	#[rstest]
	fn test_custom_limits_are_preserved() {
		// Arrange
		let limits = TsResourceLimits {
			execution_timeout: Duration::from_secs(10),
			max_source_size: 512,
			..TsResourceLimits::default()
		};

		// Act
		let runtime = TsRuntime::with_limits(limits).unwrap();

		// Assert
		assert_eq!(runtime.limits().execution_timeout, Duration::from_secs(10));
		assert_eq!(runtime.limits().max_source_size, 512);
	}

	#[rstest]
	fn test_default_limits() {
		// Arrange & Act
		let limits = TsResourceLimits::default();

		// Assert
		assert_eq!(limits.execution_timeout, DEFAULT_EXECUTION_TIMEOUT);
		assert_eq!(limits.max_source_size, DEFAULT_MAX_SOURCE_SIZE);
	}
}
