# reinhardt-manouche

Shared compiler front-end for the `reinhardt-pages` macro family.

## Overview

This crate provides the Abstract Syntax Tree (AST) structures, parsing logic,
semantic validation, and checked registries for the `page!`, `form!`, `head!`,
and `style!` DSLs. Final Rust code generation is performed downstream by
`reinhardt-pages/macros`.

The name "manouche" comes from [Manouche Jazz](https://en.wikipedia.org/wiki/Gypsy_jazz),
a genre of music created by Django Reinhardt in the 1930s.

## Modules

- `core` - DSL types, Untyped/Typed AST, reactive traits
- `parser` - TokenStream -> Untyped AST
- `style` - stable diagnostics, checked CSS registries, deterministic scoping,
  structured CSS lowering, and serialization
- `validator` - Untyped AST -> Typed AST (semantic analysis)

## Pipeline

```text
page!/form!/head!: TokenStream -> parse -> Untyped AST -> validate -> Typed AST
style!:            TokenStream -> parse -> validate -> scope -> CSS IR -> CSS
```

Manouche owns the normative style property, unit, function, and value-grammar
registries. Proc macros and static extraction both call `compile_style`, so
diagnostics, scoped names, and generated CSS cannot drift between consumers.
Failures cross the public boundary as stable `StyleDiagnosticKind` values.

Shorthand grammars preserve CSS ordering rules while accepting valid equivalent
forms. For example, `flex` accepts a basis-only value, `box-shadow` accepts
optional color and `inset` components in either order, and `background` permits
a color only on its final comma-separated layer.

The registry also keeps property-specific CSS constraints intact: flex basis
accepts `content`, maximum sizes accept intrinsic sizing keywords, grid line
names exclude reserved keywords, and `grid-template-areas` requires rectangular
named areas with consistent row widths.

Source formatting has a separate, non-semantic parser. Formatting never
replaces compilation or validation.

## Usage

Compile a style definition with the consuming package identity and authored
style type, then serialize its opaque checked stylesheet. The package name,
package version, and style type name form the deterministic scope identity.

```rust
use quote::quote;
use reinhardt_manouche::{StyleCompileContext, compile_style, serialize_css};

let compiled = compile_style(
    quote! {
        vars { accent: Color = red; }
        .card { color: vars.accent; }
    },
    &StyleCompileContext {
        package_name: "poll-app",
        package_version: "0.4.0",
        style_type_name: "PollCardStyles",
    },
)
.expect("style definition should compile");

assert_eq!(compiled.scope.suffix, "f69b9cbc74c9");
assert_eq!(compiled.classes[0].css_name, "card--rs-f69b9cbc74c9");
assert_eq!(
    serialize_css(&compiled.css),
    concat!(
        ".card--rs-f69b9cbc74c9 {\n",
        "  color: var(--rs-f69b9cbc74c9-accent, red);\n",
        "}\n",
    )
);
```

## License

BSD 3-Clause License
