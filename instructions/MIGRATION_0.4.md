# Migration Guide: 0.3.x to 0.4.0

This guide covers the breaking model identity, Reinhardt Pages event API, and
controlled form-element binding changes introduced for 0.4.

## Explicit model application labels and conventional table names

`#[model(...)]` now requires `app_label`. The previous implicit `"default"`
application label has been removed because it could silently group unrelated
models in migrations, application discovery, the model registry, and admin
configuration.

`table_name` is now optional. When omitted, Reinhardt prefixes the application
label and converts the Rust struct name to snake_case without pluralization or
English inflection:

| Struct | Default table name |
|---|---|
| `User` in `accounts` | `accounts_user` |
| `BlogPost` in `blog` | `blog_blog_post` |
| `HTTPRoute` in `routing` | `routing_http_route` |
| `Person` in `people` | `people_person` |

To preserve an existing schema, add the application label and keep the current
table name explicit:

```rust,ignore
// Before: the model was registered in the implicit "default" application.
#[model(table_name = "users")]
pub struct User {
    // Fields omitted.
}

// After: application ownership and the existing plural table are explicit.
#[model(app_label = "default", table_name = "users")]
pub struct User {
    // Fields omitted.
}
```

New models may adopt the convention by omitting only `table_name`:

```rust,ignore
#[model(app_label = "routing")]
pub struct HTTPRoute {
    // Uses the `routing_http_route` table.
}
```

Omitting an explicit `table_name` from an existing model is a schema decision,
not a source-only cleanup. For example, changing `User` in the `accounts` app
from `users` to the derived `accounts_user` name requires a table rename migration. `makemigrations`
recognizes this same-model table-name change and emits `RenameTable` instead of
destructive drop/create operations. Audit model attributes before upgrading:

```bash
rg -n '#\[model(?:\([^]]*\))?\]' src crates examples
```

For an implicit `ManyToMany` relationship, the same migration also renames the
convention-derived through table and any affected foreign-key columns. This
preserves existing relationship rows when the source model changes from a
plural table name to its singular convention-derived name. Qualified string
foreign keys such as `#[field(foreign_key = "blog.Post")]` resolve the target
model's registered `table_name`, including explicit table-name overrides.

For every result, add a meaningful `app_label`. Preserve `table_name` whenever
the deployed database already uses that table; omit it only when the derived
name is the intended schema contract.

## Reserved `bind` directive

`bind:` is now a reserved `page!` directive on supported `input`, `textarea`,
and `select` controls. It connects a typed `Signal` to the control property and
must not be used to emit a literal nonstandard HTML attribute.

```rust,ignore
// Before: emitted a nonstandard literal `bind` attribute.
page!({ input { bind: "search-model" } })

// After: prefer a standards-compatible data attribute.
page!({ input { data_bind: "search-model" } })

// Or use the low-level builder when the literal attribute name is required.
PageElement::new("input").attr("bind", "search-model")
```

For controlled state, replace imperative DOM reads with a typed signal:

```rust,ignore
let query = Signal::new(String::new());
page!({ input { aria_label: "Search", bind: query } })
```

The compiler validates the binding shape: text, radio, and single-select
controls use `Signal<String>`; checkboxes use `Signal<bool>`; multiple selects
use `Signal<Vec<String>>`; and numeric controls accept supported primitives via
direct `bind: value`. Use `bind: number(value, parse_error)` only when rejected
numeric text should also update an error signal.

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
