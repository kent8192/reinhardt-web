# Document head management

Reinhardt Pages treats document head declarations as lifecycle-managed values.
`Head` remains the value type, while the active page tree, route tree, and
retained hooks determine which declarations are currently visible.

## Public entry points

The `head!` macro supports `title`, `base { href: ... }`, `meta`, `link`,
`script`, and `style` declarations:

```rust,ignore
let head = head!(|| {
    base { href: "/app/" }
    title { "Workspace" }
    meta { name: "description", content: "Workspace" }
});
```

Attach a head value to a page with `#head:` or `Page::with_head`. A route can
contribute the same structural value with `RouteMetadata::with_head`:

```rust,ignore
let metadata = RouteMetadata::new().with_head(head!(|| {
    base { href: "/app/" }
    title { "Workspace" }
}));
```

`use_head` and `use_page_title` are retained hooks. They require explicit
dependencies such as `deps![project.clone()]`; their slot keeps its precedence
when dependencies change and is removed with the owning reactive scope.

## Resolution and ownership

Static declarations resolve in structural pre-order: a parent page contributes
before its active child, and siblings follow source order. Singleton `title`
and `base` values use last-active-wins semantics. `meta`, `link`, `style`, and
`script` collections append in order and remove only exact descriptor
duplicates. A dropped child reveals the previous active parent or layout value.

Route metadata is wrapped into the rendered page tree. Persistent layouts keep
their head registrations across sibling navigation; the leaf store owns only
the current leaf contribution. Browser history therefore restores the previous
leaf without remounting the persistent layout head node.

## SSR, streaming, and hydration

Buffered SSR and streaming shell rendering use the active resolved branch.
Unresolved or superseded candidates do not contribute head values to the shell,
and the server does not emit a later head patch. Hydration first adopts marked
SSR head nodes, then performs a disposable body-shape prepass. Durable static
head registrations and retained hooks are installed in the root or active
reactive-branch stores.

## Browser reconciliation

Only nodes carrying `data-reinhardt-head` are managed. Existing unmanaged
`title` and `base` nodes are snapshotted when first overridden and restored
when the final managed singleton disappears. Unchanged descriptors reuse their
nodes, including unchanged scripts and exact duplicate representative
transfers. Changed script descriptors use replacement nodes because a changed
script element must execute according to normal browser DOM semantics.

Node removal cannot reverse a script's already-executed side effects. Consumers
must therefore treat script side effects as irreversible and make scripts
idempotent when a route or reactive branch can be mounted more than once.
