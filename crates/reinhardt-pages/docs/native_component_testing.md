# Native component testing

The `testing` feature provides an in-memory component harness that executes the
same typed standard-event handlers used by browser mounts. It is intended for
component behavior, propagation, reactive rerenders, and deterministic async
settling without Chrome.

```rust,ignore
use reinhardt_pages::event::{ClickEvent, InputEvent};
use reinhardt_pages::prelude::*;
use reinhardt_pages::testing::component::{EventFixture, Role, render};

let name = Signal::new(String::new());
let screen = render(page!({
    input {
        aria_label: "Name",
        @input: {
            let name = name.clone();
            move |event: InputEvent| {
                name.set(event.value().expect("input value"));
            }
        },
    }
    button { @click: |event: ClickEvent| { event.prevent_default(); }, "Save" }
}));

screen
    .get_by_label("Name")
    .dispatch(EventFixture::input().value("Ada"))?;
screen.get_by_role(Role::Button, "Save").click();
screen.settle();
assert_eq!(name.get(), "Ada");
# Ok::<(), reinhardt_pages::testing::component::EventError>(())
```

## Fixtures and targets

`EventFixture::new(KnownEvent)` derives its payload family, bubbling,
cancelability, composition, and deterministic mouse defaults from the event
catalog. Convenience constructors cover `click`, `submit`, `input`, `change`,
`key_down`, and `pointer_move`. `EventFixture::custom(name)` dispatches a raw
custom event.

Target-state setters include `value`, `checked`, `selected_values`, `files`,
and `content_editable`. Validation is atomic: an invalid compound target patch
does not apply its valid fields before returning `EventError::InvalidFixture`.
The error exposes `EventFixtureError` through `std::error::Error::source`.

The originating `target()` and listener `current_target()` are distinct owned
snapshots. Bubbling creates a new current-target snapshot for each listener
while sharing propagation and default-prevention state.

## Async handlers

Typed async handlers schedule work on the screen-owned native scheduler. Call
`Screen::settle()` after dispatch to drain scheduled work and rerenders. The
method continues until tasks created by other tasks are also complete.

## Raw and component events

Use a standard `EventFixture` for intrinsic catalog events. Use
`EventFixture::custom` only with `@custom("name")` or another raw listener.
Component event props keep the argument type declared by the component prop;
they are not converted through the intrinsic event catalog.
