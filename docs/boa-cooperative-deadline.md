# Boa cooperative deadline compatibility record

## Scope and evidence

Reinhardt's host-side JavaScript runtime uses `boa_engine` 0.21.1, resolved in
`Cargo.lock`, through the optional `reinhardt-dentdelion/ts` feature. The responsible
upstream is [boa-dev/boa](https://github.com/boa-dev/boa). These observations were
reproduced on macOS aarch64 with Rust 1.96.0 on 2026-10-12.

`Script::evaluate_async_with_budget(context, 256)` yields between VM instruction
batches. Native builtin calls and property getters can reenter synchronous VM
execution before the outer future returns control. `JsValue::to_string` also
invokes guest conversion hooks synchronously. This is a compatibility limitation
for Reinhardt's elapsed-time requirement, not a claim that Boa violates its
published API contract.

Minimal inputs for a fresh context and a 20 ms cooperative polling deadline:

```javascript
// Ordinary guest call: the outer deadline regains control.
({ toString() { while (true) {} } }).toString()
// Getter and native reentry: one poll can remain active past the deadline.
({ get toString() { while (true) {} } }).toString()
({ toString() { return String({ toString() { while (true) {} } }); } }).toString()
// Calling JsValue::to_string on this completion value has the same limitation.
({ toString() { while (true) {} } })
```

The reproduction parses each input, then polls the asynchronous evaluation with a
noop waker while checking `Instant::elapsed()` before each poll. For the final
input it invokes `JsValue::to_string` after evaluation completes. Run uncapped
probes in a child process with an external timeout; the outer polling loop cannot
interrupt a synchronous reentry that never returns.

After the probe printed its evaluation-start marker, the ordinary call returned
to the deadline check in about 52 ms. Getter and nested native calls remained
active beyond a five-second child-process deadline. Setting
`context.runtime_limits_mut().set_loop_iteration_limit(1000)` stopped both in
about 6–7 ms with a runtime-limit error. These are observed local timings, not
performance guarantees. The dependency graph is `reinhardt-dentdelion` ->
`boa_engine` 0.21.1 -> Boa's VM and builtin implementations.

## Workaround and validation

`crates/reinhardt-dentdelion/src/wasm/ts_runtime.rs` captures a trusted string-hint
converter before guest evaluation and invokes bound guest methods through the
outer cooperative VM. Evaluation and conversion share the same start time. An
RAII guard clears the temporary non-enumerable global binding on every exit; an
expired evaluation retires its context. The separate configured iteration guard
remains enabled for getters and nested native reentry. It is not a wall-clock
preemption mechanism for arbitrary native work.

WASM SSR imports move blocking worker-channel waits onto Tokio's blocking pool.
The lifecycle's absolute deadline is passed through blocking-task scheduling,
queue admission, VM execution, and completion-value conversion. Component-file
loading consumes that same deadline. Expired queued work is cancelled before
execution; an admitted timeout retires the context. The WASM executor can enforce
its shorter deadline even while the interpreter is inside native work.
File metadata and bounded reads reject oversized component sources before UTF-8
decoding, pattern scanning or hydration allocation. Path/file work, props
conversion, rendering, asset extraction and hydration run in the blocking pool.
Borrowed component paths and serialized props are size-checked before their
admission copies.
Queue admission and the returned result retain the lifecycle deadline; queued
expired preparation never enters JavaScript. Hydration escaping also checks the
deadline while processing the validated source and before returning its result.
Subsequent guest calls within that lifecycle also use the remaining deadline
for epoch interruption rather than renewing the configured CPU budget. Standalone
calls receive a fresh deadline; guest entry after an expired lifecycle is denied.

Regression tests cover expensive `toString` and `Symbol.toPrimitive` methods,
string-hint semantics and receiver binding, getter/nested native iteration limits,
large finite loops, worker admission, and context retirement. Direct-worker tests
prevent a caller-side receive timeout from disguising continued interpreter work.
Generated SSR import tests also check responsiveness on a single-thread executor
with a WASM deadline shorter than the JavaScript limit and partially consumed
before import entry.

## Removal condition and reporting disposition

Replace the converter, binding bridge, and native binder only after a supported
Boa API evaluates string conversion cooperatively under the existing deadline,
including property access and nested native reentry. Verify these regression
cases with the replacement before removing the independent iteration guard.
The intended replacement is an asynchronous `ToString` operation driven by the
same deadline; such an API is not currently used or assumed to exist.

The existing upstream [cancellation feature request](https://github.com/boa-dev/boa/issues/3442)
was inspected along with the repository's contribution entrypoint on 2026-10-12.
No external report was submitted, and the full destination publication/AI policy
has not been verified. This repository record supplies internal tracking under
[UR-4 and WP-2](../instructions/UPSTREAM_ISSUE_REPORTING.md); external publication
requires the separate [PP-0](../instructions/GITHUB_INTERACTION.md#pp-0-must-external-publication)
policy and authorization checks.
