# Component-Scoped Style DSL

**Issue**: [#5568](https://github.com/kent8192/reinhardt-web/issues/5568)
**Date**: 2026-07-10
**Target line**: `develop/0.4.0`
**Status**: Approved design (pending implementation plan)

## Summary

Add a typed, component-scoped style DSL to `reinhardt-pages` with this canonical
definition form:

```rust
#[style_def]
pub(crate) static STYLES: PollCardStyles = style! {
	// Style DSL only.
};
```

`style!` owns the style language and its compiler. `#[style_def]` is a thin
item-context bridge that supplies the visibility, static name, and generated
style type name that a function-like macro cannot create from expression
position.

The selector, property, cascade, nesting, at-rule, and unit surfaces remain
recognizably CSS. The two value-language surfaces that are unusually awkward in
CSS use Rust- and JavaScript-like forms instead:

- declared, typed references replace `var(--name)`;
- operators, typed constructors, methods, tuples, and lists replace handwritten
  `calc(...)` and complex CSS function argument syntax.

The compiler generates typed class tokens and typed dynamic-variable setters.
A source scanner compiles the same style definitions into one static asset,
`__reinhardt__/components.css`. Production collection hashes that asset through
the normal manifest pipeline. Development serves it at a stable URL and uses
the existing CSS HMR protocol without rebuilding Rust or WASM when only CSS
output changes.

## Motivation

Pages applications currently keep component markup and styling in unrelated
files and pass untyped class strings through `page!`. A misspelled or deleted
class silently loses styling, generic class names share one global namespace,
and dynamic CSS custom properties require string assembly.

The existing macro stack can fail earlier. Manouche can parse and validate a
style definition, the pages macro can expose only classes and variables that
exist, and the static-files pipeline can publish the resulting CSS without
runtime injection.

The DSL must still be legible to a CSS user. Replacing selectors, properties,
or cascade rules with a Rust object model would hide the browser's actual
styling model. This design therefore changes only the variable and declaration
value syntax where a typed expression language materially improves clarity.

## Goals

- Co-locate component styles with `page!` components without embedding a CSS
  string.
- Generate deterministic, collision-resistant scoped class names.
- Make local class references compile-time checked.
- Support typed global CSS variable references and typed component variables
  with optional runtime overrides.
- Express arithmetic and common CSS functions with Rust-like operators,
  constructors, methods, and collection syntax.
- Keep selectors, properties, nesting, media queries, units, and cascade order
  CSS-like.
- Share one Manouche parser, validator, scoper, and CSS serializer between proc
  macro expansion and source extraction.
- Emit one zero-runtime-cost stylesheet through collectstatic and runserver.
- Format `style!` bodies with Topiary as a first-class Reinhardt DSL.
- Preserve existing string-valued `class:` and `style:` expressions for gradual
  adoption.

## Non-Goals

- Global resets, `:root` definitions, theme declarations, or third-party global
  overrides. Existing static CSS remains the MVP solution.
- Runtime stylesheet injection or a client-side style registry.
- TailwindCSS, UnoCSS, PostCSS, or a Node-based build step.
- Arbitrary Rust expressions inside `style!`. Dynamic values cross the boundary
  through declared component variables.
- General raw-CSS strings inside the DSL.
- Dependency-crate style discovery, workspace-wide tree shaking, style code
  splitting, or dead-style elimination.
- `@keyframes`, `@font-face`, `@supports`, `@container`, `@layer`, or a
  `global_style!` macro in the first implementation.
- User-defined style functions.
- Retrofitting `form!` with typed class tokens in this issue.
- Consolidating the existing duplicated `page!` or `form!` validation code.

## Recommended Approach

Use a CSS-shaped, structurally nested DSL with a typed value-expression layer.

The alternatives are less suitable:

| Approach | Trade-off |
|---|---|
| Structural style DSL with typed values | Preserves CSS concepts, gives deterministic parsing, and enables compile-time class and value checks |
| Raw CSS string in a proc macro | Easy to pass to a CSS parser, but loses Rust-aware tokens, typed variables, and formatter integration |
| Full Rust builder API | Strongly typed, but obscures selectors, cascade, nesting, and normal CSS review |
| Runtime CSS-in-Rust injection | Avoids extraction, but adds hydration work and creates different SSR and client style paths |
| External CSS toolchain | Mature ecosystem, but retains stringly class references and adds a separate build stack |

The structural DSL is the selected approach.

## Public Syntax

The complete representative surface is:

```rust
use reinhardt_pages::{
	CssColor, CssLength, Page, page, style, style_def,
};

#[style_def]
pub(crate) static STYLES: PollCardStyles = style! {
	globals {
		border: Color;
		surface_secondary: Color;
	}

	vars {
		padding: Length = 1rem;
		gutter: LengthPercentage = 1rem;
		height: LengthPercentage = 50vh;
		accent: Color = globals.surface_secondary;
		offset: LengthPercentage = 0px;
	}

	.card {
		padding: vars.padding;
		border: (1px, solid, globals.border);
		border-radius: 0.5rem;
		width: 100% - vars.gutter * 2;
		height: clamp(240px, vars.height, 80vh);
		color: vars.accent.mix(white, 15%);
		background: linear_gradient(Direction::Right, [
			stop(vars.accent, 0%),
			stop(vars.accent.mix(black, 20%), 100%),
		]);
		transform: (
			translate_x(vars.offset),
			rotate(6deg),
			scale(1.05),
		);

		&:hover {
			background: globals.surface_secondary;
		}

		&.featured {
			border-color: vars.accent;
		}

		> h5 {
			margin-bottom: 0.25rem;
		}

		.label {
			color: vars.accent;
		}
	}

	@media (max-width: 640px) {
		.card {
			padding: 0.75rem;
		}
	}
};

fn poll_card(accent: CssColor, padding: CssLength) -> Page {
	page!(|accent: CssColor, padding: CssLength| {
		article {
			class: STYLES.card() + STYLES.featured() + "legacy-card",
			style: STYLES.vars().accent(accent).padding(padding),
			h5 { "Poll" }
			span { class: STYLES.label(), "Open" }
		}
	})(accent, padding)
}
```

The public macros are exported by `reinhardt-pages-macros`, re-exported by
`reinhardt-pages`, and included in the pages prelude. The generated style type
has the same visibility as the annotated static.

`style!` contains only the DSL. The type name is taken from the static item, so
there is no inner `name:` field and no duplicated Rust/DSL identity.

## Macro Envelope

The MVP accepts one canonical envelope:

```rust
#[style_def]
<visibility> static <STATIC_NAME>: <SingleSegmentTypeName> = style! { ... };
```

The constraints are deliberate:

- `#[style_def]` takes no arguments and is written with the bare imported name.
- The item is an immutable `static`, not `const` or `static mut`.
- The generated type is one unqualified Rust identifier.
- The initializer is a direct, bare `style! { ... }` invocation.
- Macro aliases, qualified macro paths, `macro_rules!` wrappers, and `include!`
  are not accepted.

Rust's attribute proc-macro API passes the attribute arguments and annotated
item, but not the lexical path or delimiter used to invoke the attribute. The
attribute bridge can therefore reject non-empty arguments and invalid items,
but it cannot distinguish canonical `#[style_def]` from `#[style_def()]`, a
qualified path, or an imported alias. The source extractor owns that final
lexical check. It also reports any direct bare `style!` static initializer that
lacks the exact bare path attribute, so collectstatic and runserver fail rather
than silently omitting CSS. The non-canonical spellings are implementation
artifacts accepted by Rust expansion alone, not supported source forms.

This spelling is shared by the proc macro, source extractor, and formatter.
Keeping it exact prevents compilation, extraction, and formatting from
recognizing different programs.

Rust expands the outer attribute before the initializer macro. In the valid
form, `#[style_def]` extracts the inner tokens, invokes the shared `style!`
compiler entry, and replaces the complete item; the inner function-like macro
is not expanded a second time. The exported function-like `style!` entry uses
the same parser and validator when reached directly, then emits the missing
item-context diagnostic below. This keeps `style!` as the language/compiler
contract while limiting `#[style_def]` to item plumbing and Rust codegen.

Documentation, `cfg`, and lint attributes are propagated to the appropriate
generated type, static, and impl items. Unsupported item attributes are hard
errors rather than being copied onto an invalid Rust item. Source extraction
intentionally does not evaluate `#[cfg]`: every directly written style
definition under the selected package's `src/` tree is parsed and emitted.
Style definitions must therefore be target-neutral. Mutually exclusive `cfg`
branches cannot reuse the same package-level style identity.

If `style!` is expanded outside this envelope, it emits:

```text
style! must be the initializer of an immutable static annotated with #[style_def]
```

## Selector Semantics

Rust token streams do not preserve selector whitespace reliably. In particular,
flat `.card .label` and `.card.label` tokenise identically. The DSL therefore
never assigns meaning to selector whitespace.

Top-level style rules contain one local class root per selector branch. All
relationships and refinements are expressed through nesting:

```rust
.card {
	&:hover { /* same element pseudo-class */ }
	&[data-state="open"] { /* same element attribute */ }
	&.featured { /* same element local class */ }
	&:is(button) { /* same element type restriction */ }
	> h5 { /* direct child */ }
	+ .card { /* adjacent sibling */ }
	~ .card { /* general sibling */ }
	.label { /* descendant local class */ }
	button { /* descendant element */ }
}
```

The lowering is ordinary CSS:

| DSL | CSS shape |
|---|---|
| `.card { &:hover { ... } }` | `.card--scope:hover { ... }` |
| `.card { &.featured { ... } }` | `.card--scope.featured--scope { ... }` |
| `.card { &:is(button) { ... } }` | `.card--scope:is(button) { ... }` |
| `.card { > h5 { ... } }` | `.card--scope > h5 { ... }` |
| `.card { .label { ... } }` | `.card--scope .label--scope { ... }` |
| `.card { button { ... } }` | `.card--scope button { ... }` |

A selector head contains one simple selector. Additional same-element classes
and attributes use a nested `&` rule. A same-element type restriction uses
`&:is(button)`; forms such as `&button` are invalid. This rejects both spellings
of an ambiguous flat selector instead of silently guessing whether whitespace
was intended.

Selector lists use commas. Every top-level branch must begin with a local
class. Nested branches inherit the parent anchor, so descendant elements,
attributes, ids, universal selectors, and pseudo-classes remain component
scoped. The compiler rejects unanchored roots such as `button`, `:root`,
`html`, `body`, `*`, and `[data-theme]`.

Every class selector written anywhere in the definition is a local class and
receives a generated token accessor. There is no `:global` escape in the MVP.
Repeated rules for the same class are allowed and retain normal cascade order.

Class names are restricted to ASCII CSS identifiers that map deterministically
to Rust methods. Kebab-case is converted to snake_case for the accessor.
Definitions such as `.foo-bar` and `.foo_bar` in one style set are rejected
because both would generate `foo_bar()`. A name that maps to a Rust keyword is
also rejected with a diagnostic suggesting a non-keyword class name.

Selector pseudo-functions retain CSS selector syntax. The Rust-like function
rules in this design apply only to declaration values.

The MVP supports `@media` as a top-level grouping rule or nested inside a local
rule. Media conditions retain CSS syntax and accept static literals only;
component and global variables are not valid in media conditions.

The serializer emits flat CSS rather than browser-native nested syntax. It
walks rules in source order and splits a parent's declarations into contiguous
runs at every nested-rule or nested-at-rule boundary. For example, declarations
before a nested rule, the flattened nested rule, and declarations after it emit
as three ordered rules. This preserves cascade order even when a declaration
appears after nesting. Selector lists expand as a stable Cartesian product in
written branch order before scoping and serialization.

## Properties And Literals

Property names stay in canonical CSS kebab-case:

```rust
border-radius: 0.5rem;
margin-bottom: 0.25rem;
```

CSS numeric units also stay unchanged, including `px`, `rem`, `em`, `%`, `vh`,
`vw`, `fr`, `deg`, `ms`, and the other units registered by the style compiler.
Hex colors, CSS color keywords, quoted CSS strings in string-valued positions,
and ordinary CSS keywords remain literals rather than Rust strings containing a
whole declaration.

The checked-in Manouche style registry is the normative source for property
names, keywords, shorthand grammar, accepted value types, units, function
signatures, result types, and CSS lowering. The same table drives semantic
validation and generated public reference documentation. A registry change is
a language change and requires tests; the proc macro and extractor never carry
separate copies.

The MVP property registry contains these unprefixed property families:

| Family | Properties |
|---|---|
| Layout | `display`, `position`, `inset`, `top`, `right`, `bottom`, `left`, `float`, `clear`, `overflow`, `overflow-x`, `overflow-y`, `visibility`, `z-index` |
| Box model | `box-sizing`, `width`, `min-width`, `max-width`, `height`, `min-height`, `max-height`, `margin` and its four physical sides, `padding` and its four physical sides |
| Flex and grid | `flex`, `flex-basis`, `flex-direction`, `flex-flow`, `flex-grow`, `flex-shrink`, `flex-wrap`, `order`, `gap`, `row-gap`, `column-gap`, `align-content`, `align-items`, `align-self`, `justify-content`, `justify-items`, `justify-self`, `place-content`, `place-items`, `place-self`, `grid`, `grid-area`, `grid-auto-columns`, `grid-auto-flow`, `grid-auto-rows`, `grid-column`, `grid-row`, `grid-template`, `grid-template-areas`, `grid-template-columns`, `grid-template-rows` |
| Typography | `color`, `font`, `font-family`, `font-size`, `font-style`, `font-variant`, `font-weight`, `line-height`, `letter-spacing`, `text-align`, `text-decoration`, `text-overflow`, `text-transform`, `text-wrap`, `white-space`, `word-break` |
| Background and borders | `background`, `background-color`, `background-image`, `background-position`, `background-repeat`, `background-size`, `border`, `border-width`, `border-style`, `border-color`, their four physical-side forms, and `border-radius` plus its four corner forms |
| Effects | `box-shadow`, `opacity`, `outline`, `outline-color`, `outline-offset`, `outline-style`, `outline-width`, `filter`, `backdrop-filter` |
| Transform and transition | `transform`, `transform-origin`, `transition`, `transition-property`, `transition-duration`, `transition-timing-function`, `transition-delay` |
| Interaction and generated content | `cursor`, `pointer-events`, `resize`, `touch-action`, `user-select`, `content`, `list-style`, `list-style-position`, `list-style-type` |

Logical-property variants and additional stable CSS properties are additive
registry entries after the MVP. Vendor-prefixed and unknown properties are hard
errors. Experimental or unsupported declarations remain in ordinary static CSS
until promoted into the registry.

The MVP unit registry is:

| Dimension | Units |
|---|---|
| Absolute length | `px`, `cm`, `mm`, `q`, `in`, `pc`, `pt` |
| Font-relative length | `em`, `rem`, `ex`, `rex`, `cap`, `rcap`, `ch`, `rch`, `ic`, `ric`, `lh`, `rlh` |
| Viewport length | `vw`, `vh`, `vi`, `vb`, `vmin`, `vmax` and their `sv`, `lv`, and `dv` variants |
| Container length | `cqw`, `cqh`, `cqi`, `cqb`, `cqmin`, `cqmax` |
| Grid fraction | `fr` |
| Angle | `deg`, `grad`, `rad`, `turn` |
| Time | `ms`, `s` |
| Percentage | `%` |

Numbers and integers are unitless. A literal `0` is contextually accepted for
any numeric dimension; a nonzero unitless literal remains `Number` or `Integer`
and cannot stand in for a length, angle, time, or percentage.

Direct custom-property declarations are not accepted inside rules. Typed
`globals` and `vars` are the custom-property surface.

## Global And Component Variables

`globals` declares typed references to CSS custom properties owned outside the
component:

```rust
globals {
	border: Color;
	surface_secondary: Color;
}
```

It emits no custom-property definitions. Identifiers are converted from
snake_case to CSS kebab-case:

```text
globals.border            -> var(--border)
globals.surface_secondary -> var(--surface-secondary)
```

`vars` declares style-set-owned custom properties:

```rust
vars {
	padding: Length = 1rem;
	accent: Color = globals.surface_secondary;
}
```

Every component variable has a type and a required default. References lower
to a scoped custom property with the compiled default as its fallback:

```text
vars.padding -> var(--rs-<scope>-padding, 1rem)
vars.accent  -> var(--rs-<scope>-accent, var(--surface-secondary))
```

Defaults are placed at each reference rather than declared on every local
class. Declaring defaults on descendant classes would override a value inherited
from an ancestor's inline style. Fallback lowering preserves normal custom
property inheritance without requiring a designated root class.

Variable defaults may reference globals and other component variables in any
declaration order. Manouche builds the dependency graph, validates types, and
rejects cycles.

The initial variable type vocabulary is:

| DSL type | Runtime override type | Meaning |
|---|---|---|
| `Color` | `CssColor` | Parsed CSS color |
| `Length` | `CssLength` | Length without percentage |
| `LengthPercentage` | `CssLengthPercentage` | Length or percentage |
| `Percentage` | `CssPercentage` | Percentage |
| `Angle` | `CssAngle` | Angle |
| `Time` | `CssTime` | Duration |
| `Number` | `CssNumber` | Finite scalar |
| `Integer` | `CssInteger` | Integer scalar |

Runtime wrapper constructors validate their values and serialize one CSS value;
they do not accept an unchecked declaration string.

For every component variable, the macro generates a consuming builder method:

```rust
let style_vars = STYLES
	.vars()
	.accent(accent)
	.padding(CssLength::rem(0.75));
```

`StyleVars` serializes only the selected inline overrides:

```css
--rs-a1b2c3d4e5f6-accent: <value>;
--rs-a1b2c3d4e5f6-padding: 0.75rem;
```

Calling one generated setter again replaces that variable's previous override.
Serialization emits at most one declaration per variable in the source order of
the `vars` block, independent of setter call order. An empty builder serializes
to an empty attribute value.

An unset variable uses its compiled fallback. A nonexistent setter is a Rust
type error.

## Function And Expression Semantics

The numeric type lattice is fixed:

| Operands | Result |
|---|---|
| `Integer` with `Integer` under `+`, `-`, or `*` | `Integer` |
| `Integer` with `Number` | `Number` |
| two values of the same dimension | that dimension |
| `Length` with `Percentage` | `LengthPercentage` |
| `LengthPercentage` with `Length`, `Percentage`, or `LengthPercentage` | `LengthPercentage` |

No other cross-dimension promotion exists. In particular, angle, time, color,
and keyword values never mix through arithmetic. Unary `+` and `-` preserve the
operand type. A signed atomic literal emits directly; negating a compound
expression participates in `calc(...)` lowering.

Declaration values use normal operator precedence:

- `*` and `/` bind before `+` and `-`;
- a single parenthesized expression groups arithmetic;
- addition and subtraction require compatible dimensions;
- multiplication accepts two scalars or one dimensioned value and one scalar;
- division accepts a scalar or dimensioned numerator and a scalar divisor,
  with a literal zero rejected;
- dimension-by-dimension multiplication and division are rejected.

Scalar division returns `Number`; dimension-by-scalar division preserves the
dimension. The same promotion table applies across arguments to `min`, `max`,
and `clamp`.

Any expression containing arithmetic lowers to `calc(...)`. Atomic values and
ordinary typed function calls do not receive a redundant `calc` wrapper:

```text
100% - vars.gutter * 2
-> calc(100% - var(--rs-<scope>-gutter, 1rem) * 2)
```

Writing `calc(...)` directly is rejected with a diagnostic to use operators.
Writing `var(...)` directly is rejected with a diagnostic to declare a
`global` or component `var`.

Value collections use two structural forms:

| DSL | CSS output |
|---|---|
| `(1px, solid, vars.border)` | space-separated sequence |
| `[first, second, third]` | comma-separated list |
| `slash(first, second)` | slash-separated pair |

A one-item `(expression)` is arithmetic grouping. A multi-item parenthesized
form is a space-separated sequence.

Functions use a registry rather than generic text rewriting. The registry maps
Rust-style names to canonical CSS spellings, argument types, and result types.
For example, `linear_gradient` becomes `linear-gradient`, while `translate_x`
becomes `translateX`.

The initial function registry is:

| DSL signature | Result | CSS lowering |
|---|---|---|
| `min(T, T, ...)`, at least two arguments | joined numeric `T` | `min(...)` |
| `max(T, T, ...)`, at least two arguments | joined numeric `T` | `max(...)` |
| `clamp(T, T, T)` | joined numeric `T` | `clamp(min, preferred, max)` |
| `Color::rgb(NumberOrPercentage, NumberOrPercentage, NumberOrPercentage)` | `Color` | modern `rgb(r g b)` |
| `Color::hsl(Angle, Percentage, Percentage)` | `Color` | modern `hsl(h s l)` |
| `Color::oklch(NumberOrPercentage, Number, Angle)` | `Color` | `oklch(l c h)` |
| `color.mix(other: Color, amount: Percentage)` | `Color` | `color-mix(in srgb, self calc(100% - amount), other amount)` |
| `stop(Color, LengthPercentage)` | `GradientStop` | `<color> <position>` |
| `linear_gradient(Direction, [GradientStop, ...])` with at least two stops | `Image` | `linear-gradient(<direction>, <comma-separated stops>)` |
| `translate(LengthPercentage, LengthPercentage)` | `TransformFunction` | `translate(x, y)` |
| `translate_x(LengthPercentage)` | `TransformFunction` | `translateX(x)` |
| `translate_y(LengthPercentage)` | `TransformFunction` | `translateY(y)` |
| `rotate(Angle)` | `TransformFunction` | `rotate(angle)` |
| `scale(Number)` | `TransformFunction` | `scale(number)` |
| `scale_x(Number)` | `TransformFunction` | `scaleX(number)` |
| `scale_y(Number)` | `TransformFunction` | `scaleY(number)` |
| `slash(A, B)` | `SlashPair<A, B>` | `<A> / <B>` |

`Direction` initially contains `Top`, `TopRight`, `Right`, `BottomRight`,
`Bottom`, `BottomLeft`, `Left`, and `TopLeft`, lowered to the corresponding CSS
`to ...` phrase.

For `color.mix(other, amount)`, `amount` is the weight of `other`; the receiver
has the complementary `100% - amount` weight. The MVP fixes the interpolation
space to sRGB so output does not depend on an implicit browser choice.

Receiver methods are type checked. For example, `.mix(...)` is available only
on a color expression and returns `Color`.

An unregistered function is a compile error. A newly introduced browser
function can be used explicitly while the registry catches up:

```rust
background: unchecked_fn!(paint(my_worklet));
```

`unchecked_fn!` is a special style-DSL escape, not an arbitrary Rust macro. It
accepts one balanced function call, preserves structural token boundaries, and
produces the explicit opaque type `Unchecked`. It is valid only as an entire
property value or as the entire default of a typed component variable. For a
property, it skips that declaration's value-compatibility check while still
requiring a registered property name. For a variable, the declared variable
type supplies the runtime contract. `Unchecked` cannot participate in
arithmetic, receiver methods, tuples, lists, or checked function arguments.
There is no general raw declaration escape; ordinary static CSS is the final
fallback.

## Generated Rust API

The attribute bridge generates:

- the requested zero-sized style type;
- the annotated static value;
- one class accessor for every local class;
- one generated component-variable builder type, such as
  `PollCardStylesVars`;
- typed builder methods for declared component variables.

The shared runtime types live in `reinhardt-pages`:

| Type | Contract |
|---|---|
| `ClassToken` | `Copy` token holding one scoped class name |
| `ClassList` | Ordered class composition with one space between non-empty entries |
| `StyleVars` | Validated inline custom-property declarations |
| `CssColor`, `CssLength`, and peers | Typed dynamic CSS values |

`STYLES.vars()` returns the generated builder. The builder owns a `StyleVars`
buffer, supports direct setter chaining, and is itself convertible to the
attribute value expected by `page!`; no terminal `build()` call is required.

Class composition supports at least:

- `ClassToken + ClassToken -> ClassList`;
- `ClassList + ClassToken -> ClassList`;
- `ClassToken + &'static str -> ClassList`;
- `ClassList + &'static str -> ClassList`.

Composition preserves order, skips empty string entries, and does not reorder or
deduplicate classes.

`page!` currently lowers ordinary attribute expressions through
`Cow::from(value_expr)`. `From<ClassToken>`, `From<ClassList>`, and
`From<StyleVars>` for `Cow<'static, str>`, together with the corresponding
conversion generated for each style-variable builder, make the generated
values work in the existing `class:` and `style:` attributes without a new
attribute syntax. Plain strings remain supported and remain outside the style
compiler's class guarantees.

## Compiler Architecture

The compiler follows the proven `page!` stage shape while making Manouche the
single semantic implementation for this new DSL:

```text
style tokens
    -> Manouche untyped style AST
    -> Manouche validation and typed style AST
    -> Manouche scoping and CSS IR
       -> pages macro Rust codegen
       -> extractor CSS serialization
```

Responsibilities are divided as follows:

| Crate | Responsibility |
|---|---|
| `reinhardt-manouche` | Untyped and typed style ASTs, parser, property/function registries, semantic validation, scoping, CSS IR, deterministic CSS serialization |
| `reinhardt-pages-macros` | `style!` compiler entry, `#[style_def]` item bridge, and Rust API codegen from typed/scoped output |
| `reinhardt-pages` | `ClassToken`, `ClassList`, `StyleVars`, runtime CSS value wrappers, and stylesheet URL helper |
| `reinhardt-commands` | Current-package source scanning, virtual static asset generation, collectstatic integration, and runserver/HMR integration |
| `reinhardt-formatter` | Macro detection and Topiary orchestration |
| `tree-sitter-reinhardt-style` | Concrete syntax tree used only by formatting/editor tooling |

`page!` currently demonstrates
`TokenStream -> Untyped AST -> Typed AST -> Rust codegen`, but some page
validation remains duplicated between Manouche and the macro crate. The style
implementation does not copy that duplication. The macro crate performs Rust
codegen only after Manouche has produced validated, scoped output.

The Topiary tree-sitter grammar is intentionally a formatter parser, not a
second semantic compiler. It never decides whether a style is valid.

The proc macro does not write files. The extractor invokes the same Manouche
compiler and serializer, so class names and generated CSS cannot drift from the
Rust API.

## Scoping Contract

The scope identity is the NUL-separated byte sequence:

```text
rstyle-v1\0<Cargo package name>\0<Cargo package version>\0<style type name>
```

The package values come from `CARGO_PKG_NAME` and `CARGO_PKG_VERSION` during
macro expansion and the selected `cargo_metadata::Package` during extraction.
The extractor does not read raw workspace-inherited version fields from TOML.

The first 12 lowercase hexadecimal characters of SHA-256 over that identity are
the public scope suffix. Generated names are:

```text
class:    <local-css-name>--rs-<12-hex-scope>
variable: --rs-<12-hex-scope>-<variable-kebab-name>
```

The identity excludes module paths, source paths, absolute paths, line numbers,
and CSS content. Moving a module or editing declarations therefore does not
change class names. Adding or removing a local class changes the generated Rust
API but not existing class names.

Within one style definition, Manouche detects accessor collisions. Across the
selected package, the extractor rejects duplicate full identities and rejects
distinct identities whose shortened hashes collide. A proc macro invocation
cannot observe other modules, so package-wide failures are extraction-time
errors in collectstatic and runserver.

Changing the scope format or identity inputs requires a new version prefix such
as `rstyle-v2`.

## Source Extraction

`StyleExtractor` scans one selected application package's `src/**/*.rs` files
using `syn`. Collectstatic and runserver resolve and share one
`StylePackageContext` containing the Cargo manifest path and Cargo package ID.
Selection follows this order:

1. an explicit `--package <name>` selection resolved through Cargo metadata;
2. otherwise `cargo_metadata::Metadata::root_package()` for the command's
   manifest path;
3. otherwise a hard error requiring `--package`.

An unknown package name, a package outside the loaded metadata graph, or more
than one package matching the requested name is a hard error. A virtual
workspace root without `--package` has no root package and therefore does not
guess among workspace members. `CollectStaticCommand` receives the resolved
context as an explicit builder input rather than deriving it from static-files
directories. Collectstatic and runserver expose the same optional `--package`
argument and pass the same resolved context into extraction.

Within that package, the scanner recognizes only the canonical direct envelope:

```text
bare #[style_def]
    + immutable ItemStatic
    + single-segment generated type
    + initializer Expr::Macro with bare path style
```

The scanner also detects direct bare `style!` static initializers and attribute
paths ending in `style_def` that do not match this envelope. Those are hard
errors. This is required because the attribute proc-macro API cannot observe
whether its own invocation used a qualified path, alias, or empty parentheses.

The scanner obtains the package name, version, manifest path, and source root
from the selected Cargo metadata entry. Files are parsed as Rust; the scanner
does not use regexes to find macros.

Definitions are sorted by package name and style type name. Rules and
declarations inside one definition retain source order. The resulting bundle is
serialized deterministically. Rust source comments are not included in the CSS
asset because proc-macro token streams do not carry comments.

The scanner always owns one logical asset, including when the package contains
zero style definitions:

```text
__reinhardt__/components.css
```

An empty current bundle replaces the previous bundle so deleting the last style
cannot leave stale rules active.

The MVP does not discover aliases, wrappers, `include!`, generated Rust files,
or dependency package definitions. A library that ships component styles needs
dependency discovery in a later design before those styles can be consumed
transitively.

## Collectstatic Integration

`CollectStaticCommand` gains a virtual-asset input that accepts a logical path
and bytes. Component CSS enters this path before manifest serialization and
before explicit `index.html` template processing.

Extraction, package-wide identity checks, and CSS serialization complete before
the command mutates the destination or manifest. A compiler or collision error
therefore leaves the previous collected output intact and makes collectstatic
fail.

The virtual asset follows normal static-file behavior:

- production hashing creates a content-hashed filename;
- the logical-to-hashed mapping is written to `manifest.json`;
- copy/unmodified statistics include the generated asset;
- dry-run parses, validates, and reports the asset without writing it;
- `--link` still writes generated bytes because an in-memory asset cannot be a
  symlink;
- ignore patterns cannot suppress the reserved framework asset;
- old generated development output and old `components.<hash>.css` variants are
  pruned after a successful replacement;
- a physical source claiming `__reinhardt__/components.css` is a hard collision
  error.

The `__reinhardt__/` logical namespace is reserved for framework-managed
assets.

Pure-WASM applications add one explicit link to their source index:

```html
<link
  rel="stylesheet"
  href='{{ static_url("__reinhardt__/components.css") }}'
>
```

The explicit index source is rendered after the manifest is complete, so
production output contains the hashed URL. An index containing template
placeholders is rendered rather than symlinked even when `--link` is selected.

SSR applications add one explicit link through the existing static resolver:

```rust
head!(|| {
	link {
		rel: "stylesheet",
		href: component_stylesheet_url(),
	}
})
```

`component_stylesheet_url()` is a small wrapper around
`resolve_static("__reinhardt__/components.css")`. It does not auto-inject a
head element. Applications include it once per document.

The resolver is initialized from the same `manifest.json` mapping written by
`CollectStaticCommand`; this feature does not introduce a second manifest name
or format. Production integration tests must prove that the helper returns the
hashed asset that actually exists.

## Development And HMR

Runserver generates the current bundle once before the development child server
starts. The autoreload parent owns subsequent generation, and the child does not
duplicate it. Pages mode still performs the startup generation with
`--noreload`, `--no-wasm`, `--no-wasm-rebuild`, or `--no-collectstatic`; those
flags suppress their named build or collection work, not the development
stylesheet required by the page.

An initial style extraction failure aborts Pages runserver startup because no
last-good generated asset exists yet. After startup, watcher failures preserve
the last-good asset as described below.

Development writes atomically to a framework-managed generated-asset root and
mounts that root at the active `STATIC_URL` in both runserver implementations.
It does not write generated files into a user-owned source `static/` directory.
The physical asset path is:

```text
<generated-static-root>/__reinhardt__/components.css
```

Its stable development URL is `join_static_url(STATIC_URL, logical_path)`. With
the default static prefix it is:

```text
/static/__reinhardt__/components.css
```

The generated-asset mount has higher priority than user static sources for the
reserved `__reinhardt__/` namespace. The runserver index renderer resolves the
component stylesheet placeholder to the active development URL before serving
the SPA index. It never serves the literal `{{ static_url(...) }}` expression.
SSR development initializes the static resolver with the same
logical-to-development URL mapping.

After a successful replacement, runserver sends the existing message:

```rust
HmrMessage::CssUpdate {
	path: "__reinhardt__/components.css".to_string(),
}
```

The current HMR client matches the link suffix and adds a cache-busting query,
so no new protocol variant is required.

Style generation is an independent runserver stage. It runs before both the
`RebuildTargets::has_work()` early return and the static `page!` hot-patch early
return, including when WASM rebuilding is disabled.

The watcher keeps an initial snapshot and updates it after every successful
style compilation. Each recognized source file contributes three fingerprints:

| Fingerprint | Contents |
|---|---|
| non-style Rust | Rust tokens with each style body replaced by a stable envelope marker |
| generated Rust API | Scope identity, class accessors, component variable setters, and variable types |
| CSS | Deterministic serialized bundle bytes |

The dispatch rules are:

1. If only the CSS fingerprint changes, atomically replace the bundle, send
   `CssUpdate`, and skip server and WASM rebuilds.
2. If the non-style Rust or generated Rust API fingerprint changes, update the
   bundle and continue through the normal server/WASM target classifier.
3. If style parsing or validation fails, report source diagnostics, retain the
   last-good CSS asset and snapshot, and fail closed into the normal applicable
   rebuild path.
4. Removing the final style emits the empty bundle and a CSS update.

Adding or removing a class or component variable is therefore not treated as a
CSS-only edit, even though the change occurs inside `style!`.

## Formatter And Editor Integration

Formatting is implemented with Topiary from the first release:

- add `MacroKind::Style` to `reinhardt-formatter`;
- detect bare `style!` invocations as `("style", MacroKind::Style)`;
- add `crates/reinhardt-formatter/queries/style_formatting.scm`;
- add the `tree-sitter-reinhardt-style` workspace crate;
- route `MacroKind::Style` to `tree_sitter_reinhardt_style::LANGUAGE`.

The outer attribute and static item are Rust and remain rustfmt-owned. The
`style!` body is protected from rustfmt and formatted by Topiary. The style
formatter preserves comments, declaration order, rule order, and the original
macro envelope. It does not parse and reserialize the body through the semantic
CSS IR.

The `reinhardt-admin` binary's existing `fmt` and `fmt-all` subcommands in
`reinhardt-admin-cli` delegate to the `reinhardt-formatter` binary, so they gain
the same style grammar and query without a duplicated admin-side query.

Required formatter properties are idempotence, comment preservation, stable
indentation for nested selectors and at-rules, and correct formatting for typed
expressions, tuples, lists, constructors, methods, and `unchecked_fn!`.

## Diagnostics

Proc-macro diagnostics use token spans. Extractor diagnostics add the source
path and source location. The two paths use the same Manouche error kinds and
wording.

Required hard errors include:

- an unsupported `#[style_def]` item envelope;
- standalone `style!` expansion;
- an unanchored top-level selector;
- a flat or otherwise ambiguous selector head;
- a class-to-accessor collision;
- duplicate variable or global names within their namespace;
- an undeclared `vars` or `globals` reference;
- a missing component-variable default;
- a component-variable dependency cycle;
- a property/value type mismatch;
- invalid arithmetic dimensions;
- an unknown property or function;
- direct `var(...)` or `calc(...)` use;
- an invalid function argument or receiver method;
- duplicate package-level style identity;
- a shortened scope-hash collision;
- a physical file collision with the reserved generated asset.

Error messages prefer a concrete supported rewrite. For example, a flat
descendant error shows the nested selector form, direct `var()` points to
`globals` or `vars`, and an unknown function points to `unchecked_fn!`.

Repeated CSS rules for one class are not duplicate declarations and remain
valid.

## Testing Strategy

### Manouche

- Parser tests for globals, vars, rules, structural nesting, media queries,
  typed expressions, methods, tuples, lists, and unchecked functions.
- Validator tests for anchoring, accessor collisions, property types, function
  signatures, arithmetic dimensions, variable dependencies, and cycles.
- Registry conformance tests for every normative property, unit, function
  signature, result type, and CSS lowering.
- Golden CSS IR and serialization tests, including fallback custom properties
  and parent-declaration splitting around nested boundaries.
- Scope-vector tests that lock the `rstyle-v1` SHA-256 contract.

### Pages Macros And Runtime

- Trybuild pass tests for the canonical envelope, generated class methods,
  class composition, and dynamic variable builders.
- Trybuild fail tests for every envelope and semantic diagnostic category.
- Tests for `ClassToken`, `ClassList`, and `StyleVars` conversion to
  `Cow<'static, str>`.
- Tests proving plain string classes continue to compile.
- A parity test proving macro-generated class names equal extractor-generated
  selector names.

### Extraction And Static Files

- Package-selection tests for explicit package names, root packages, virtual
  workspace ambiguity, unknown packages, and shared runserver/collectstatic
  context.
- Source scanner tests for direct definitions, nested modules, target-neutral
  `cfg` input, and unsupported indirection.
- Deterministic ordering and full-bundle golden tests.
- Duplicate identity and shortened hash collision tests.
- Virtual-asset tests for hashing, `manifest.json`, dry-run, `--link`, stats,
  physical collisions, empty bundles, and stale hashed-file removal.
- Index processing tests proving `static_url` resolves after the generated asset
  enters the manifest.
- Resolver integration proving `component_stylesheet_url()` names an existing
  hashed production file.

### Formatter

- Topiary golden tests for every selector and value form.
- Idempotence tests over already formatted input.
- Comment, rule-order, and declaration-order preservation tests.
- Tests proving the outer static remains rustfmt-owned.
- Admin CLI delegation coverage using a file containing `style!`.

### Runserver And Rendering

- Startup generation with autoreload, `--noreload`, `--no-wasm`, and
  `--no-wasm-rebuild`.
- Pipeline-order tests proving styles run before both existing early returns.
- Fingerprint tests distinguishing CSS-only changes from generated API and
  ordinary Rust changes.
- Last-good asset retention after compiler failure.
- Atomic replacement, custom `STATIC_URL`, and `CssUpdate` path matching.
- A CSS-only edit test proving neither server nor WASM rebuild runs.
- Pure-WASM index coverage and SSR coverage for class, inline variable, and
  explicit stylesheet link output.

## Documentation

Implementation updates include:

- `reinhardt-pages` crate docs and README for the public DSL and runtime types;
- the pages prelude documentation;
- formatter documentation and CLI help, which currently list only `page!`,
  `form!`, and `head!`;
- collectstatic and runserver documentation for generated component CSS;
- a migration example that converts one reference component from string classes
  while retaining mixed string/token composition.

Documentation explains that selectors and properties remain CSS, shows why
descendants use nesting, and presents globals, defaulted component variables,
operators, and dynamic overrides before advanced functions.

## Implementation Boundaries

- Manouche is the sole semantic compiler for style definitions.
- The proc macro never writes an asset.
- The extractor never substitutes a second parser or scoping algorithm.
- Formatter parsing never decides semantic validity.
- Scope identity never depends on local filesystem location or CSS content.
- Dynamic declaration values enter only through typed component variables.
- Generated defaults use custom-property fallbacks, not declarations copied to
  every class.
- Production and development use the same logical asset path and compiled CSS
  bytes; only URL hashing differs.
- Failed development compilation never replaces last-good CSS.
- Existing string classes and static CSS remain supported.
- Global CSS, dependency scanning, keyframes, code splitting, tree shaking, and
  user-defined functions require separate designs.

## Decisions Closed By This Spec

| Decision | Choice |
|---|---|
| Definition boundary | `#[style_def] static NAME: Type = style! { ... };` |
| Compiler owner | `style!` language compiled semantically by Manouche; thin item bridge in the pages macro crate |
| Selector model | CSS nesting semantics with structural descendants and no whitespace-significant flat selectors |
| Property model | CSS kebab-case properties with one normative checked-in typed registry |
| Variable model | Typed `globals` plus defaulted, scoped component `vars` |
| Default lowering | Fallback at every `vars.name` reference |
| Dynamic overrides | Generated typed builder serialized through the existing `style:` attribute |
| Expression model | Rust-like operators, typed constructors and methods, space tuples, comma lists |
| Escape hatch | Typed-context `unchecked_fn!` only; no general raw declaration string |
| Class use | `ClassToken` and `ClassList` through the existing `class:` attribute |
| Scope identity | `rstyle-v1` + Cargo package name/version + generated style type |
| Extraction | Explicit/root Cargo package selection, then `src/**/*.rs` scan of the direct canonical envelope |
| Static output | One virtual `__reinhardt__/components.css` asset |
| Production delivery | Normal content hash and `manifest.json` resolution |
| SSR delivery | Explicit one-time `component_stylesheet_url()` link |
| Development delivery | Stable `STATIC_URL`-derived URL, atomic replacement, existing CSS HMR message |
| CSS-only rebuild behavior | Three fingerprints; skip server/WASM only when Rust and generated API are unchanged |
| Formatting | Dedicated tree-sitter grammar and Topiary query through `reinhardt-formatter` |
| Deferred surfaces | Global styles, dependency scan, keyframes, code splitting, tree shaking, custom functions, and typed `form!` classes |

Refs #5568.
