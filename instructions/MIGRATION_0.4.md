# Migration Guide: 0.3.x to 0.4.0

This guide currently covers the breaking Reinhardt Pages event API introduced
for 0.4. Add later 0.4 migration topics as their public contracts stabilize.

## Typed intrinsic events

Standard intrinsic `page!` handlers no longer receive one raw event type.
Each catalog event selects an exact payload such as `ClickEvent`, `InputEvent`,
or `ChangeEvent`.

```rust,ignore
// Before
fn handle_input(event: reinhardt_pages::platform::Event) {
    // Browser-only target cast.
}

// After
fn handle_input(event: reinhardt_pages::event::InputEvent) {
    match event.value() {
        Ok(value) => save(value),
        Err(error) => report(error),
    }
}
```

Inferred closures normally need no annotation:

```rust,ignore
page!({ input { @input: |event| { let _ = event.value(); } } })
```

External functions and `Callback` values must use the exact payload selected by
the event name. A payload for another event is a compile-time error.

## Raw handlers and custom events

Use explicit raw adapters when low-level access is required:

```rust,ignore
use reinhardt_pages::{raw_event_handler, platform};

let handler = raw_event_handler(|event: platform::Event| inspect(event));
```

Arbitrary intrinsic names use `@custom("name")` and receive
`platform::Event`. The 0.4 event API does not add typed custom detail values;
that follow-up is tracked by #5636. Browser-only raw APIs remain available
through `payload.raw()` on WASM, but portable code should prefer payload
methods and owned target snapshots.

## Target extraction

Replace `event.target()` casts and unchecked `expect` calls with capability
methods. `value`, `checked`, `selected_values`, and `files` return
`Result<_, EventTargetError>`. They read the listener's captured
`current_target`, not an element recast after async work begins.

## Native events and tests

`DummyEvent` is removed. Low-level native handlers receive `NativeEvent`, while
standard handlers receive the same generated payload types as WASM. Enable the
`testing` feature and use `EventFixture` to supply family data and target state.
Call `Screen::settle()` after async handlers or reactive writes. See
[`native_component_testing.md`](../crates/reinhardt-pages/docs/native_component_testing.md).

## Low-level event names

`reinhardt_core::types::page::EventType` now aliases the complete catalog-backed
`KnownEvent` enum. Code that exhaustively matched the previous small enum must
handle the expanded standard event set. Use `EventName` when a value may be
either a catalog event or an explicit custom name.

Parsing a standard name now returns `UnknownEventName` instead of `()`:

```rust,ignore
use reinhardt_core::types::page::EventType;

let event = "click".parse::<EventType>()?;
let dom_name = event.as_str();
```

The former `From<EventType> for &'static str` conversion is removed. Replace
`let name: &'static str = event.into();` with `event.as_str()`.

## Component event props

Component `@event` props are not intrinsic DOM events. Keep the component prop's
declared domain type, `()`, or an explicit standard payload when that is truly
the component contract. `@custom("name")` is valid only on intrinsic elements.

## Migration scan

```bash
rg -n "DummyEvent|platform::Event|event\.target\(\)|dyn_into::<.*Html" src crates examples
```

Classify intentional raw custom-event and low-level integration code before
replacing it. Then run native component tests and a WASM target check.
