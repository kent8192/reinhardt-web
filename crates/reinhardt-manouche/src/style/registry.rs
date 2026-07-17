//! Immutable registries for checked component styles.

use std::fmt::Write;

use crate::core::{
	KeywordDomain, NumericConstraint, NumericDimension, SemanticType, TypeConstraint,
};
use crate::style::diagnostic::StyleDiagnosticKind;

/// A semantic grouping within the CSS unit registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnitCategory {
	/// An absolute length unit.
	AbsoluteLength,
	/// A font-relative length unit.
	FontRelativeLength,
	/// A viewport-relative length unit.
	ViewportLength,
	/// A query-container-relative length unit.
	ContainerLength,
	/// A grid fraction unit.
	GridFraction,
	/// An angle unit.
	Angle,
	/// A duration unit.
	Time,
	/// The percentage unit.
	Percentage,
}

/// One registered CSS numeric unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UnitSpec {
	/// The suffix as written in the style DSL.
	pub name: &'static str,
	/// The numeric dimension produced by the unit.
	pub dimension: NumericDimension,
	/// The unit's semantic grouping.
	pub category: UnitCategory,
}

const fn unit(name: &'static str, dimension: NumericDimension, category: UnitCategory) -> UnitSpec {
	UnitSpec {
		name,
		dimension,
		category,
	}
}

static UNIT_SPECS: &[UnitSpec] = &[
	unit("px", NumericDimension::Length, UnitCategory::AbsoluteLength),
	unit("cm", NumericDimension::Length, UnitCategory::AbsoluteLength),
	unit("mm", NumericDimension::Length, UnitCategory::AbsoluteLength),
	unit("q", NumericDimension::Length, UnitCategory::AbsoluteLength),
	unit("in", NumericDimension::Length, UnitCategory::AbsoluteLength),
	unit("pc", NumericDimension::Length, UnitCategory::AbsoluteLength),
	unit("pt", NumericDimension::Length, UnitCategory::AbsoluteLength),
	unit(
		"em",
		NumericDimension::Length,
		UnitCategory::FontRelativeLength,
	),
	unit(
		"rem",
		NumericDimension::Length,
		UnitCategory::FontRelativeLength,
	),
	unit(
		"ex",
		NumericDimension::Length,
		UnitCategory::FontRelativeLength,
	),
	unit(
		"rex",
		NumericDimension::Length,
		UnitCategory::FontRelativeLength,
	),
	unit(
		"cap",
		NumericDimension::Length,
		UnitCategory::FontRelativeLength,
	),
	unit(
		"rcap",
		NumericDimension::Length,
		UnitCategory::FontRelativeLength,
	),
	unit(
		"ch",
		NumericDimension::Length,
		UnitCategory::FontRelativeLength,
	),
	unit(
		"rch",
		NumericDimension::Length,
		UnitCategory::FontRelativeLength,
	),
	unit(
		"ic",
		NumericDimension::Length,
		UnitCategory::FontRelativeLength,
	),
	unit(
		"ric",
		NumericDimension::Length,
		UnitCategory::FontRelativeLength,
	),
	unit(
		"lh",
		NumericDimension::Length,
		UnitCategory::FontRelativeLength,
	),
	unit(
		"rlh",
		NumericDimension::Length,
		UnitCategory::FontRelativeLength,
	),
	unit("vw", NumericDimension::Length, UnitCategory::ViewportLength),
	unit("vh", NumericDimension::Length, UnitCategory::ViewportLength),
	unit("vi", NumericDimension::Length, UnitCategory::ViewportLength),
	unit("vb", NumericDimension::Length, UnitCategory::ViewportLength),
	unit(
		"vmin",
		NumericDimension::Length,
		UnitCategory::ViewportLength,
	),
	unit(
		"vmax",
		NumericDimension::Length,
		UnitCategory::ViewportLength,
	),
	unit(
		"svw",
		NumericDimension::Length,
		UnitCategory::ViewportLength,
	),
	unit(
		"svh",
		NumericDimension::Length,
		UnitCategory::ViewportLength,
	),
	unit(
		"svi",
		NumericDimension::Length,
		UnitCategory::ViewportLength,
	),
	unit(
		"svb",
		NumericDimension::Length,
		UnitCategory::ViewportLength,
	),
	unit(
		"svmin",
		NumericDimension::Length,
		UnitCategory::ViewportLength,
	),
	unit(
		"svmax",
		NumericDimension::Length,
		UnitCategory::ViewportLength,
	),
	unit(
		"lvw",
		NumericDimension::Length,
		UnitCategory::ViewportLength,
	),
	unit(
		"lvh",
		NumericDimension::Length,
		UnitCategory::ViewportLength,
	),
	unit(
		"lvi",
		NumericDimension::Length,
		UnitCategory::ViewportLength,
	),
	unit(
		"lvb",
		NumericDimension::Length,
		UnitCategory::ViewportLength,
	),
	unit(
		"lvmin",
		NumericDimension::Length,
		UnitCategory::ViewportLength,
	),
	unit(
		"lvmax",
		NumericDimension::Length,
		UnitCategory::ViewportLength,
	),
	unit(
		"dvw",
		NumericDimension::Length,
		UnitCategory::ViewportLength,
	),
	unit(
		"dvh",
		NumericDimension::Length,
		UnitCategory::ViewportLength,
	),
	unit(
		"dvi",
		NumericDimension::Length,
		UnitCategory::ViewportLength,
	),
	unit(
		"dvb",
		NumericDimension::Length,
		UnitCategory::ViewportLength,
	),
	unit(
		"dvmin",
		NumericDimension::Length,
		UnitCategory::ViewportLength,
	),
	unit(
		"dvmax",
		NumericDimension::Length,
		UnitCategory::ViewportLength,
	),
	unit(
		"cqw",
		NumericDimension::Length,
		UnitCategory::ContainerLength,
	),
	unit(
		"cqh",
		NumericDimension::Length,
		UnitCategory::ContainerLength,
	),
	unit(
		"cqi",
		NumericDimension::Length,
		UnitCategory::ContainerLength,
	),
	unit(
		"cqb",
		NumericDimension::Length,
		UnitCategory::ContainerLength,
	),
	unit(
		"cqmin",
		NumericDimension::Length,
		UnitCategory::ContainerLength,
	),
	unit(
		"cqmax",
		NumericDimension::Length,
		UnitCategory::ContainerLength,
	),
	unit(
		"fr",
		NumericDimension::GridFraction,
		UnitCategory::GridFraction,
	),
	unit("deg", NumericDimension::Angle, UnitCategory::Angle),
	unit("grad", NumericDimension::Angle, UnitCategory::Angle),
	unit("rad", NumericDimension::Angle, UnitCategory::Angle),
	unit("turn", NumericDimension::Angle, UnitCategory::Angle),
	unit("ms", NumericDimension::Time, UnitCategory::Time),
	unit("s", NumericDimension::Time, UnitCategory::Time),
	unit("%", NumericDimension::Percentage, UnitCategory::Percentage),
];

/// Returns the complete immutable MVP unit registry.
pub fn unit_specs() -> &'static [UnitSpec] {
	UNIT_SPECS
}

// This crate-private lookup is the validation boundary for parsed numeric literals.
#[allow(dead_code)]
pub(crate) fn unit_spec(name: &str) -> Option<&'static UnitSpec> {
	UNIT_SPECS.iter().find(|spec| spec.name == name)
}

/// One of the eight closed MVP property families.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PropertyFamily {
	/// Layout and positioning properties.
	Layout,
	/// Box sizing and physical spacing properties.
	BoxModel,
	/// Flexbox and grid properties.
	FlexAndGrid,
	/// Text and font properties.
	Typography,
	/// Background and physical border properties.
	BackgroundAndBorder,
	/// Visual effect properties.
	Effects,
	/// Transform and transition properties.
	TransformAndTransition,
	/// Interaction and generated-content properties.
	InteractionAndGenerated,
}

/// One member of an ordered or unordered structural grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GrammarMember {
	/// The stable role used by validation and lowering.
	pub role: &'static str,
	/// The member grammar.
	pub grammar: &'static ValueGrammar,
	/// Whether the member may be omitted.
	pub optional: bool,
}

/// A data-driven grammar for one registered property value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueGrammar {
	/// One primitive semantic type.
	Primitive(SemanticType),
	/// One primitive grammar whose literal value cannot be negative.
	NonNegative(&'static ValueGrammar),
	/// One numeric grammar whose literal value must fall within an inclusive range.
	NumericRange {
		/// The underlying numeric grammar.
		grammar: &'static ValueGrammar,
		/// The inclusive lower bound.
		minimum: i16,
		/// The inclusive upper bound.
		maximum: i16,
	},
	/// One member of an explicit keyword domain.
	Keyword(&'static KeywordDomain),
	/// A validated CSS custom identifier.
	Identifier,
	/// A validated CSS custom identifier excluding reserved values.
	IdentifierExcept(&'static [&'static str]),
	/// A value produced by an approved typed function.
	FunctionResult(SemanticType),
	/// A choice among complete alternative grammars.
	Or(&'static [ValueGrammar]),
	/// A bounded or unbounded space-separated repetition.
	Space {
		/// The minimum item count.
		min: usize,
		/// The maximum item count, or no upper bound.
		max: Option<usize>,
		/// The grammar repeated for each item.
		item: &'static ValueGrammar,
	},
	/// A comma-separated repetition.
	Comma {
		/// The minimum item count.
		min: usize,
		/// The grammar repeated for each item.
		item: &'static ValueGrammar,
	},
	/// A comma-separated repetition whose final item has a distinct grammar.
	CommaFinal {
		/// The minimum item count.
		min: usize,
		/// The grammar repeated for every item before the final item.
		item: &'static ValueGrammar,
		/// The grammar used by the final item.
		final_item: &'static ValueGrammar,
	},
	/// A slash-separated pair.
	Slash {
		/// The grammar before the slash.
		left: &'static ValueGrammar,
		/// The grammar after the slash.
		right: &'static ValueGrammar,
	},
	/// A bounded slash-separated repetition.
	SlashList {
		/// The minimum item count.
		min: usize,
		/// The maximum item count.
		max: usize,
		/// The grammar repeated for each item.
		item: &'static ValueGrammar,
	},
	/// Members consumed in the declared order.
	Ordered(&'static [GrammarMember]),
	/// Members consumed in any valid order.
	Unordered {
		/// The available members and their lowering roles.
		members: &'static [GrammarMember],
		/// The minimum number of present members.
		min_members: usize,
		/// Whether lowering preserves authored member order.
		preserve_source_order: bool,
	},
}

impl ValueGrammar {
	/// Returns a deterministic structural description of this grammar.
	pub fn describe(&self) -> String {
		match self {
			Self::Primitive(semantic_type) | Self::FunctionResult(semantic_type) => {
				semantic_type_name(*semantic_type).to_owned()
			}
			Self::NonNegative(grammar) => format!("NON_NEGATIVE({})", grammar.describe()),
			Self::NumericRange {
				grammar,
				minimum,
				maximum,
			} => format!("RANGE({minimum},{maximum},{})", grammar.describe()),
			Self::Keyword(domain) => format!("KW({})", domain.name),
			Self::Identifier => "IDENT".to_owned(),
			Self::IdentifierExcept(excluded) => format!("IDENT_EXCEPT({})", excluded.join("|")),
			Self::Or(alternatives) => format!(
				"OR({})",
				alternatives
					.iter()
					.map(Self::describe)
					.collect::<Vec<_>>()
					.join(",")
			),
			Self::Space { min, max, item } => format!(
				"SPACE({min},{},{})",
				max.map_or_else(|| "*".to_owned(), |value| value.to_string()),
				item.describe()
			),
			Self::Comma { min, item } => format!("COMMA({min},{})", item.describe()),
			Self::CommaFinal {
				min,
				item,
				final_item,
			} => format!(
				"COMMA_FINAL({min},{},{})",
				item.describe(),
				final_item.describe()
			),
			Self::Slash { left, right } => {
				format!("SLASH({},{})", left.describe(), right.describe())
			}
			Self::SlashList { min, max, item } => {
				format!("SLASH_LIST({min},{max},{})", item.describe())
			}
			Self::Ordered(members) => format!("ORDERED({})", describe_members(members)),
			Self::Unordered {
				members,
				min_members,
				preserve_source_order,
			} => format!(
				"UNORDERED(min={min_members},source-order={preserve_source_order},{})",
				describe_members(members)
			),
		}
	}

	fn describe_with_keywords(&self) -> String {
		match self {
			Self::Primitive(semantic_type) | Self::FunctionResult(semantic_type) => {
				semantic_type_name(*semantic_type).to_owned()
			}
			Self::NonNegative(grammar) => {
				format!("NON_NEGATIVE({})", grammar.describe_with_keywords())
			}
			Self::NumericRange {
				grammar,
				minimum,
				maximum,
			} => format!(
				"RANGE({minimum},{maximum},{})",
				grammar.describe_with_keywords()
			),
			Self::Keyword(domain) => format!("KW({}:[{}])", domain.name, domain.keywords.join("|")),
			Self::Identifier => "IDENT".to_owned(),
			Self::IdentifierExcept(excluded) => format!("IDENT_EXCEPT({})", excluded.join("|")),
			Self::Or(alternatives) => format!(
				"OR({})",
				alternatives
					.iter()
					.map(Self::describe_with_keywords)
					.collect::<Vec<_>>()
					.join(",")
			),
			Self::Space { min, max, item } => format!(
				"SPACE({min},{},{})",
				max.map_or_else(|| "*".to_owned(), |value| value.to_string()),
				item.describe_with_keywords()
			),
			Self::Comma { min, item } => {
				format!("COMMA({min},{})", item.describe_with_keywords())
			}
			Self::CommaFinal {
				min,
				item,
				final_item,
			} => format!(
				"COMMA_FINAL({min},{},{})",
				item.describe_with_keywords(),
				final_item.describe_with_keywords()
			),
			Self::Slash { left, right } => format!(
				"SLASH({},{})",
				left.describe_with_keywords(),
				right.describe_with_keywords()
			),
			Self::SlashList { min, max, item } => {
				format!("SLASH_LIST({min},{max},{})", item.describe_with_keywords())
			}
			Self::Ordered(members) => {
				format!("ORDERED({})", describe_members_with_keywords(members))
			}
			Self::Unordered {
				members,
				min_members,
				preserve_source_order,
			} => format!(
				"UNORDERED(min={min_members},source-order={preserve_source_order},{})",
				describe_members_with_keywords(members)
			),
		}
	}
}

fn semantic_type_name(semantic_type: SemanticType) -> &'static str {
	match semantic_type {
		SemanticType::Color => "COLOR",
		SemanticType::Length => "LENGTH",
		SemanticType::LengthPercentage => "LENGTH_PERCENTAGE",
		SemanticType::Percentage => "PERCENTAGE",
		SemanticType::Angle => "ANGLE",
		SemanticType::Time => "TIME",
		SemanticType::Number => "NUMBER",
		SemanticType::Integer => "INTEGER",
		SemanticType::GridFraction => "GRID_FRACTION",
		SemanticType::QuotedString => "QUOTED_STRING",
		SemanticType::CustomIdentifier => "CUSTOM_IDENTIFIER",
		SemanticType::Keyword => "KEYWORD",
		SemanticType::Direction => "DIRECTION",
		SemanticType::GradientStop => "GRADIENT_STOP",
		SemanticType::Image => "IMAGE",
		SemanticType::TransformFunction => "TRANSFORM_FUNCTION",
		SemanticType::SpaceSequence => "SPACE_SEQUENCE",
		SemanticType::CommaList => "COMMA_LIST",
		SemanticType::SlashPair => "SLASH_PAIR",
		SemanticType::Unchecked => "UNCHECKED",
	}
}

fn describe_members(members: &[GrammarMember]) -> String {
	members
		.iter()
		.map(|member| {
			format!(
				"{}{}:{}",
				member.role,
				if member.optional { "?" } else { "" },
				member.grammar.describe()
			)
		})
		.collect::<Vec<_>>()
		.join(",")
}

fn describe_members_with_keywords(members: &[GrammarMember]) -> String {
	members
		.iter()
		.map(|member| {
			format!(
				"{}{}:{}",
				member.role,
				if member.optional { "?" } else { "" },
				member.grammar.describe_with_keywords()
			)
		})
		.collect::<Vec<_>>()
		.join(",")
}

/// One immutable property-registry entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PropertySpec {
	/// The canonical unprefixed CSS property name.
	pub name: &'static str,
	/// The property's MVP family.
	pub family: PropertyFamily,
	/// The complete non-CSS-wide structural value grammar.
	pub grammar: &'static ValueGrammar,
	/// The CSS-wide keyword domain accepted in addition to `grammar`.
	pub css_wide_keywords: &'static KeywordDomain,
}

const fn required(role: &'static str, grammar: &'static ValueGrammar) -> GrammarMember {
	GrammarMember {
		role,
		grammar,
		optional: false,
	}
}

const fn optional(role: &'static str, grammar: &'static ValueGrammar) -> GrammarMember {
	GrammarMember {
		role,
		grammar,
		optional: true,
	}
}

macro_rules! keyword_grammar {
	($domain:ident, $grammar:ident, $name:literal, [$($keyword:literal),+ $(,)?]) => {
		static $domain: KeywordDomain = KeywordDomain {
			name: $name,
			keywords: &[$($keyword),+],
			produced_type: SemanticType::Keyword,
		};
		const $grammar: ValueGrammar = ValueGrammar::Keyword(&$domain);
	};
}

static CSS_WIDE_DOMAIN: KeywordDomain = KeywordDomain {
	name: "css-wide",
	keywords: &["inherit", "initial", "unset", "revert", "revert-layer"],
	produced_type: SemanticType::Keyword,
};
static NAMED_COLOR_DOMAIN: KeywordDomain = KeywordDomain {
	name: "named-color",
	keywords: &[
		"aliceblue",
		"antiquewhite",
		"aqua",
		"aquamarine",
		"azure",
		"beige",
		"bisque",
		"black",
		"blanchedalmond",
		"blue",
		"blueviolet",
		"brown",
		"burlywood",
		"cadetblue",
		"chartreuse",
		"chocolate",
		"coral",
		"cornflowerblue",
		"cornsilk",
		"crimson",
		"cyan",
		"darkblue",
		"darkcyan",
		"darkgoldenrod",
		"darkgray",
		"darkgreen",
		"darkgrey",
		"darkkhaki",
		"darkmagenta",
		"darkolivegreen",
		"darkorange",
		"darkorchid",
		"darkred",
		"darksalmon",
		"darkseagreen",
		"darkslateblue",
		"darkslategray",
		"darkslategrey",
		"darkturquoise",
		"darkviolet",
		"deeppink",
		"deepskyblue",
		"dimgray",
		"dimgrey",
		"dodgerblue",
		"firebrick",
		"floralwhite",
		"forestgreen",
		"fuchsia",
		"gainsboro",
		"ghostwhite",
		"gold",
		"goldenrod",
		"gray",
		"green",
		"greenyellow",
		"grey",
		"honeydew",
		"hotpink",
		"indianred",
		"indigo",
		"ivory",
		"khaki",
		"lavender",
		"lavenderblush",
		"lawngreen",
		"lemonchiffon",
		"lightblue",
		"lightcoral",
		"lightcyan",
		"lightgoldenrodyellow",
		"lightgray",
		"lightgreen",
		"lightgrey",
		"lightpink",
		"lightsalmon",
		"lightseagreen",
		"lightskyblue",
		"lightslategray",
		"lightslategrey",
		"lightsteelblue",
		"lightyellow",
		"lime",
		"limegreen",
		"linen",
		"magenta",
		"maroon",
		"mediumaquamarine",
		"mediumblue",
		"mediumorchid",
		"mediumpurple",
		"mediumseagreen",
		"mediumslateblue",
		"mediumspringgreen",
		"mediumturquoise",
		"mediumvioletred",
		"midnightblue",
		"mintcream",
		"mistyrose",
		"moccasin",
		"navajowhite",
		"navy",
		"oldlace",
		"olive",
		"olivedrab",
		"orange",
		"orangered",
		"orchid",
		"palegoldenrod",
		"palegreen",
		"paleturquoise",
		"palevioletred",
		"papayawhip",
		"peachpuff",
		"peru",
		"pink",
		"plum",
		"powderblue",
		"purple",
		"rebeccapurple",
		"red",
		"rosybrown",
		"royalblue",
		"saddlebrown",
		"salmon",
		"sandybrown",
		"seagreen",
		"seashell",
		"sienna",
		"silver",
		"skyblue",
		"slateblue",
		"slategray",
		"slategrey",
		"snow",
		"springgreen",
		"steelblue",
		"tan",
		"teal",
		"thistle",
		"tomato",
		"transparent",
		"turquoise",
		"violet",
		"wheat",
		"white",
		"whitesmoke",
		"yellow",
		"yellowgreen",
		"currentcolor",
	],
	produced_type: SemanticType::Color,
};

pub(crate) fn named_color_domain() -> &'static KeywordDomain {
	&NAMED_COLOR_DOMAIN
}

// Named-keyword inference is consumed by semantic validation after registry construction.
#[allow(dead_code)]
pub(crate) fn infer_named_keyword_type(keyword: &str) -> Option<SemanticType> {
	let domain = named_color_domain();
	domain
		.keywords
		.iter()
		.any(|candidate| candidate.eq_ignore_ascii_case(keyword))
		.then_some(domain.produced_type)
}

keyword_grammar!(
	DISPLAY_DOMAIN,
	KW_DISPLAY,
	"display",
	[
		"none",
		"contents",
		"block",
		"inline",
		"inline-block",
		"flow-root",
		"flex",
		"inline-flex",
		"grid",
		"inline-grid",
		"table",
		"table-row",
		"table-cell",
		"list-item",
	]
);
keyword_grammar!(
	POSITION_MODE_DOMAIN,
	KW_POSITION_MODE,
	"position-mode",
	["static", "relative", "absolute", "fixed", "sticky",]
);
keyword_grammar!(AUTO_DOMAIN, KW_AUTO, "auto", ["auto"]);
keyword_grammar!(NONE_DOMAIN, KW_NONE, "none", ["none"]);
keyword_grammar!(NORMAL_DOMAIN, KW_NORMAL, "normal", ["normal"]);
keyword_grammar!(
	FLOAT_DOMAIN,
	KW_FLOAT,
	"float",
	["none", "left", "right", "inline-start", "inline-end",]
);
keyword_grammar!(
	CLEAR_DOMAIN,
	KW_CLEAR,
	"clear",
	[
		"none",
		"left",
		"right",
		"inline-start",
		"inline-end",
		"both",
	]
);
keyword_grammar!(
	OVERFLOW_DOMAIN,
	KW_OVERFLOW,
	"overflow",
	["visible", "hidden", "clip", "scroll", "auto",]
);
keyword_grammar!(
	VISIBILITY_DOMAIN,
	KW_VISIBILITY,
	"visibility",
	["visible", "hidden", "collapse",]
);
keyword_grammar!(
	BOX_SIZING_DOMAIN,
	KW_BOX_SIZING,
	"box-sizing",
	["content-box", "border-box",]
);
keyword_grammar!(
	SIZE_DOMAIN,
	KW_SIZE,
	"size",
	[
		"auto",
		"min-content",
		"max-content",
		"fit-content",
		"stretch",
	]
);
keyword_grammar!(
	MAX_SIZE_DOMAIN,
	KW_MAX_SIZE,
	"max-size",
	["min-content", "max-content", "fit-content", "stretch"]
);
keyword_grammar!(FLEX_BASIS_DOMAIN, KW_FLEX_BASIS, "flex-basis", ["content"]);
keyword_grammar!(FLEX_DOMAIN, KW_FLEX, "flex", ["none", "auto", "initial"]);
keyword_grammar!(
	FLEX_DIRECTION_DOMAIN,
	KW_FLEX_DIRECTION,
	"flex-direction",
	["row", "row-reverse", "column", "column-reverse",]
);
keyword_grammar!(
	FLEX_WRAP_DOMAIN,
	KW_FLEX_WRAP,
	"flex-wrap",
	["nowrap", "wrap", "wrap-reverse",]
);
keyword_grammar!(
	ALIGN_CONTENT_DOMAIN,
	KW_ALIGN_CONTENT,
	"align-content",
	[
		"normal",
		"start",
		"end",
		"flex-start",
		"flex-end",
		"center",
		"space-between",
		"space-around",
		"space-evenly",
		"stretch",
		"baseline",
	]
);
keyword_grammar!(
	ALIGN_ITEMS_DOMAIN,
	KW_ALIGN_ITEMS,
	"align-items",
	[
		"normal",
		"stretch",
		"start",
		"end",
		"self-start",
		"self-end",
		"flex-start",
		"flex-end",
		"center",
		"baseline",
	]
);
keyword_grammar!(
	ALIGN_SELF_DOMAIN,
	KW_ALIGN_SELF,
	"align-self",
	[
		"normal",
		"stretch",
		"start",
		"end",
		"self-start",
		"self-end",
		"flex-start",
		"flex-end",
		"center",
		"baseline",
		"auto",
	]
);
keyword_grammar!(
	JUSTIFY_CONTENT_DOMAIN,
	KW_JUSTIFY_CONTENT,
	"justify-content",
	[
		"normal",
		"start",
		"end",
		"flex-start",
		"flex-end",
		"center",
		"space-between",
		"space-around",
		"space-evenly",
		"stretch",
		"baseline",
		"left",
		"right",
	]
);
keyword_grammar!(
	JUSTIFY_ITEMS_DOMAIN,
	KW_JUSTIFY_ITEMS,
	"justify-items",
	[
		"normal",
		"stretch",
		"start",
		"end",
		"self-start",
		"self-end",
		"left",
		"right",
		"center",
		"legacy",
	]
);
keyword_grammar!(
	JUSTIFY_SELF_DOMAIN,
	KW_JUSTIFY_SELF,
	"justify-self",
	[
		"normal",
		"stretch",
		"start",
		"end",
		"self-start",
		"self-end",
		"left",
		"right",
		"center",
		"auto",
	]
);
keyword_grammar!(
	TRACK_DOMAIN,
	KW_TRACK,
	"track",
	["auto", "min-content", "max-content",]
);
keyword_grammar!(
	GRID_FLOW_AXIS_DOMAIN,
	KW_GRID_FLOW_AXIS,
	"grid-flow-axis",
	["row", "column",]
);
keyword_grammar!(DENSE_DOMAIN, KW_DENSE, "dense", ["dense"]);
keyword_grammar!(SPAN_DOMAIN, KW_SPAN, "span", ["span"]);
keyword_grammar!(
	FONT_SIZE_DOMAIN,
	KW_FONT_SIZE,
	"font-size",
	[
		"xx-small",
		"x-small",
		"small",
		"medium",
		"large",
		"x-large",
		"xx-large",
		"xxx-large",
		"smaller",
		"larger",
	]
);
keyword_grammar!(
	FONT_STYLE_DOMAIN,
	KW_FONT_STYLE,
	"font-style",
	["normal", "italic"]
);
keyword_grammar!(OBLIQUE_DOMAIN, KW_OBLIQUE, "oblique", ["oblique"]);
keyword_grammar!(
	FONT_VARIANT_DOMAIN,
	KW_FONT_VARIANT,
	"font-variant",
	["normal", "small-caps",]
);
keyword_grammar!(
	FONT_WEIGHT_DOMAIN,
	KW_FONT_WEIGHT,
	"font-weight",
	["normal", "bold", "bolder", "lighter",]
);
keyword_grammar!(
	SYSTEM_FONT_DOMAIN,
	KW_SYSTEM_FONT,
	"system-font",
	[
		"caption",
		"icon",
		"menu",
		"message-box",
		"small-caption",
		"status-bar",
	]
);
keyword_grammar!(
	TEXT_ALIGN_DOMAIN,
	KW_TEXT_ALIGN,
	"text-align",
	[
		"start",
		"end",
		"left",
		"right",
		"center",
		"justify",
		"match-parent",
	]
);
keyword_grammar!(
	TEXT_OVERFLOW_DOMAIN,
	KW_TEXT_OVERFLOW,
	"text-overflow",
	["clip", "ellipsis",]
);
keyword_grammar!(
	TEXT_DECORATION_LINE_DOMAIN,
	KW_TEXT_DECORATION_LINE,
	"text-decoration-line",
	["underline", "overline", "line-through",]
);
keyword_grammar!(
	TEXT_DECORATION_STYLE_DOMAIN,
	KW_TEXT_DECORATION_STYLE,
	"text-decoration-style",
	["solid", "double", "dotted", "dashed", "wavy",]
);
keyword_grammar!(
	TEXT_DECORATION_THICKNESS_DOMAIN,
	KW_TEXT_DECORATION_THICKNESS,
	"text-decoration-thickness",
	["auto", "from-font",]
);
keyword_grammar!(
	TEXT_TRANSFORM_DOMAIN,
	KW_TEXT_TRANSFORM,
	"text-transform",
	[
		"none",
		"capitalize",
		"uppercase",
		"lowercase",
		"full-width",
		"full-size-kana",
	]
);
keyword_grammar!(
	TEXT_WRAP_DOMAIN,
	KW_TEXT_WRAP,
	"text-wrap",
	["wrap", "nowrap", "balance", "pretty", "stable",]
);
keyword_grammar!(
	WHITE_SPACE_DOMAIN,
	KW_WHITE_SPACE,
	"white-space",
	[
		"normal",
		"pre",
		"pre-wrap",
		"pre-line",
		"nowrap",
		"break-spaces",
	]
);
keyword_grammar!(
	WORD_BREAK_DOMAIN,
	KW_WORD_BREAK,
	"word-break",
	["normal", "break-all", "keep-all", "break-word",]
);
keyword_grammar!(
	POSITION_VALUE_DOMAIN,
	KW_POSITION_VALUE,
	"position",
	["left", "center", "right", "top", "bottom",]
);
keyword_grammar!(
	HORIZONTAL_POSITION_DOMAIN,
	KW_HORIZONTAL_POSITION,
	"horizontal-position",
	["left", "center", "right",]
);
keyword_grammar!(
	VERTICAL_POSITION_DOMAIN,
	KW_VERTICAL_POSITION,
	"vertical-position",
	["top", "center", "bottom",]
);
keyword_grammar!(
	HORIZONTAL_POSITION_EDGE_DOMAIN,
	KW_HORIZONTAL_POSITION_EDGE,
	"horizontal-position-edge",
	["left", "right",]
);
keyword_grammar!(
	VERTICAL_POSITION_EDGE_DOMAIN,
	KW_VERTICAL_POSITION_EDGE,
	"vertical-position-edge",
	["top", "bottom",]
);
keyword_grammar!(
	LINE_WIDTH_DOMAIN,
	KW_LINE_WIDTH,
	"line-width",
	["thin", "medium", "thick",]
);
keyword_grammar!(
	LINE_STYLE_DOMAIN,
	KW_LINE_STYLE,
	"line-style",
	[
		"none", "hidden", "dotted", "dashed", "solid", "double", "groove", "ridge", "inset",
		"outset",
	]
);
keyword_grammar!(
	OUTLINE_STYLE_DOMAIN,
	KW_OUTLINE_STYLE,
	"outline-style",
	[
		"none", "dotted", "dashed", "solid", "double", "groove", "ridge", "inset", "outset",
		"auto",
	]
);
keyword_grammar!(
	BACKGROUND_REPEAT_DOMAIN,
	KW_BACKGROUND_REPEAT,
	"background-repeat",
	[
		"repeat",
		"repeat-x",
		"repeat-y",
		"space",
		"round",
		"no-repeat",
	]
);
keyword_grammar!(
	BACKGROUND_REPEAT_PAIR_DOMAIN,
	KW_BACKGROUND_REPEAT_PAIR,
	"background-repeat-pair",
	["repeat", "space", "round", "no-repeat",]
);
keyword_grammar!(
	BACKGROUND_SIZE_DOMAIN,
	KW_BACKGROUND_SIZE,
	"background-size",
	["cover", "contain", "auto",]
);
keyword_grammar!(INSET_DOMAIN, KW_INSET, "inset", ["inset"]);
keyword_grammar!(INVERT_DOMAIN, KW_INVERT, "invert", ["invert"]);
keyword_grammar!(
	TRANSITION_PROPERTY_DOMAIN,
	KW_TRANSITION_PROPERTY,
	"transition-property",
	["all", "none",]
);
keyword_grammar!(
	TRANSITION_PROPERTY_LIST_DOMAIN,
	KW_TRANSITION_PROPERTY_LIST,
	"transition-property-list",
	["all",]
);
keyword_grammar!(
	TIMING_DOMAIN,
	KW_TIMING,
	"timing",
	[
		"linear",
		"ease",
		"ease-in",
		"ease-out",
		"ease-in-out",
		"step-start",
		"step-end",
	]
);
keyword_grammar!(
	CURSOR_DOMAIN,
	KW_CURSOR,
	"cursor",
	[
		"auto",
		"default",
		"none",
		"context-menu",
		"help",
		"pointer",
		"progress",
		"wait",
		"cell",
		"crosshair",
		"text",
		"vertical-text",
		"alias",
		"copy",
		"move",
		"no-drop",
		"not-allowed",
		"grab",
		"grabbing",
		"all-scroll",
		"col-resize",
		"row-resize",
		"n-resize",
		"e-resize",
		"s-resize",
		"w-resize",
		"ne-resize",
		"nw-resize",
		"se-resize",
		"sw-resize",
		"ew-resize",
		"ns-resize",
		"nesw-resize",
		"nwse-resize",
		"zoom-in",
		"zoom-out",
	]
);
keyword_grammar!(
	POINTER_EVENTS_DOMAIN,
	KW_POINTER_EVENTS,
	"pointer-events",
	[
		"auto",
		"none",
		"visiblePainted",
		"visibleFill",
		"visibleStroke",
		"visible",
		"painted",
		"fill",
		"stroke",
		"bounding-box",
		"all",
	]
);
keyword_grammar!(
	RESIZE_DOMAIN,
	KW_RESIZE,
	"resize",
	["none", "both", "horizontal", "vertical", "block", "inline",]
);
keyword_grammar!(
	TOUCH_ACTION_DOMAIN,
	KW_TOUCH_ACTION,
	"touch-action",
	["auto", "none", "manipulation",]
);
keyword_grammar!(
	TOUCH_X_GESTURE_DOMAIN,
	KW_TOUCH_X_GESTURE,
	"touch-x-gesture",
	["pan-x", "pan-left", "pan-right",]
);
keyword_grammar!(
	TOUCH_Y_GESTURE_DOMAIN,
	KW_TOUCH_Y_GESTURE,
	"touch-y-gesture",
	["pan-y", "pan-up", "pan-down",]
);
keyword_grammar!(
	TOUCH_PINCH_ZOOM_DOMAIN,
	KW_TOUCH_PINCH_ZOOM,
	"touch-pinch-zoom",
	["pinch-zoom",]
);
keyword_grammar!(
	USER_SELECT_DOMAIN,
	KW_USER_SELECT,
	"user-select",
	["auto", "text", "none", "contain", "all",]
);
keyword_grammar!(CONTENT_DOMAIN, KW_CONTENT, "content", ["normal", "none"]);
keyword_grammar!(
	LIST_POSITION_DOMAIN,
	KW_LIST_POSITION,
	"list-style-position",
	["inside", "outside",]
);
keyword_grammar!(
	LIST_TYPE_DOMAIN,
	KW_LIST_TYPE,
	"list-style-type",
	[
		"none",
		"disc",
		"circle",
		"square",
		"decimal",
		"decimal-leading-zero",
		"lower-roman",
		"upper-roman",
		"lower-alpha",
		"upper-alpha",
	]
);

const N: ValueGrammar = ValueGrammar::Primitive(SemanticType::Number);
const I: ValueGrammar = ValueGrammar::Primitive(SemanticType::Integer);
const L: ValueGrammar = ValueGrammar::Primitive(SemanticType::Length);
const LP: ValueGrammar = ValueGrammar::Primitive(SemanticType::LengthPercentage);
const NN: ValueGrammar = ValueGrammar::NonNegative(&N);
const NL: ValueGrammar = ValueGrammar::NonNegative(&L);
const NLP: ValueGrammar = ValueGrammar::NonNegative(&LP);
const FONT_WEIGHT_NUMBER: ValueGrammar = ValueGrammar::NumericRange {
	grammar: &N,
	minimum: 1,
	maximum: 1000,
};
const P: ValueGrammar = ValueGrammar::Primitive(SemanticType::Percentage);
const A: ValueGrammar = ValueGrammar::Primitive(SemanticType::Angle);
const T: ValueGrammar = ValueGrammar::Primitive(SemanticType::Time);
const NT: ValueGrammar = ValueGrammar::NonNegative(&T);
const C: ValueGrammar = ValueGrammar::Primitive(SemanticType::Color);
const S: ValueGrammar = ValueGrammar::Primitive(SemanticType::QuotedString);
const FR: ValueGrammar = ValueGrammar::Primitive(SemanticType::GridFraction);
const NFR: ValueGrammar = ValueGrammar::NonNegative(&FR);
const IMG: ValueGrammar = ValueGrammar::FunctionResult(SemanticType::Image);
const TF: ValueGrammar = ValueGrammar::FunctionResult(SemanticType::TransformFunction);
const IDENT: ValueGrammar = ValueGrammar::Identifier;

const SIZE: ValueGrammar = ValueGrammar::Or(&[NLP, KW_SIZE]);
const POSITIVE_INTEGER: ValueGrammar = ValueGrammar::NumericRange {
	grammar: &I,
	minimum: 1,
	maximum: i16::MAX,
};
const MAX_SIZE: ValueGrammar = ValueGrammar::Or(&[NLP, KW_MAX_SIZE, KW_NONE]);
const POSITION_ITEM: ValueGrammar = ValueGrammar::Or(&[LP, KW_POSITION_VALUE]);
const HORIZONTAL_POSITION: ValueGrammar = ValueGrammar::Or(&[LP, KW_HORIZONTAL_POSITION]);
const VERTICAL_POSITION: ValueGrammar = ValueGrammar::Or(&[LP, KW_VERTICAL_POSITION]);
const POSITION_TWO: ValueGrammar = ValueGrammar::Unordered {
	members: &[
		required("horizontal", &HORIZONTAL_POSITION),
		required("vertical", &VERTICAL_POSITION),
	],
	min_members: 2,
	preserve_source_order: true,
};
const POSITION_THREE: ValueGrammar = ValueGrammar::Or(&[
	ValueGrammar::Ordered(&[
		required("horizontal-edge", &KW_HORIZONTAL_POSITION_EDGE),
		required("horizontal-offset", &LP),
		required("vertical", &VERTICAL_POSITION),
	]),
	ValueGrammar::Ordered(&[
		required("vertical-edge", &KW_VERTICAL_POSITION_EDGE),
		required("vertical-offset", &LP),
		required("horizontal", &HORIZONTAL_POSITION),
	]),
	ValueGrammar::Ordered(&[
		required("horizontal", &HORIZONTAL_POSITION),
		required("vertical-edge", &KW_VERTICAL_POSITION_EDGE),
		required("vertical-offset", &LP),
	]),
	ValueGrammar::Ordered(&[
		required("vertical", &VERTICAL_POSITION),
		required("horizontal-edge", &KW_HORIZONTAL_POSITION_EDGE),
		required("horizontal-offset", &LP),
	]),
]);
const POSITION_FOUR: ValueGrammar = ValueGrammar::Or(&[
	ValueGrammar::Ordered(&[
		required("horizontal-edge", &KW_HORIZONTAL_POSITION_EDGE),
		required("horizontal-offset", &LP),
		required("vertical-edge", &KW_VERTICAL_POSITION_EDGE),
		required("vertical-offset", &LP),
	]),
	ValueGrammar::Ordered(&[
		required("vertical-edge", &KW_VERTICAL_POSITION_EDGE),
		required("vertical-offset", &LP),
		required("horizontal-edge", &KW_HORIZONTAL_POSITION_EDGE),
		required("horizontal-offset", &LP),
	]),
]);
const POSITION: ValueGrammar =
	ValueGrammar::Or(&[POSITION_ITEM, POSITION_TWO, POSITION_THREE, POSITION_FOUR]);
const TRANSFORM_ORIGIN_POSITION: ValueGrammar = ValueGrammar::Or(&[POSITION_ITEM, POSITION_TWO]);
const LINE_WIDTH: ValueGrammar = ValueGrammar::Or(&[NL, KW_LINE_WIDTH]);
const LINE_STYLE: ValueGrammar = KW_LINE_STYLE;
const BORDER: ValueGrammar = ValueGrammar::Unordered {
	members: &[
		optional("width", &LINE_WIDTH),
		optional("style", &LINE_STYLE),
		optional("color", &C),
	],
	min_members: 1,
	preserve_source_order: false,
};
const GRID_LINE_NAME: ValueGrammar = ValueGrammar::IdentifierExcept(&["auto", "span"]);
const GRID_LINE_NUMBER_AND_NAME: ValueGrammar = ValueGrammar::Unordered {
	members: &[required("number", &I), required("name", &GRID_LINE_NAME)],
	min_members: 2,
	preserve_source_order: true,
};
const POSITIVE_GRID_LINE_NUMBER_AND_NAME: ValueGrammar = ValueGrammar::Unordered {
	members: &[
		required("number", &POSITIVE_INTEGER),
		required("name", &GRID_LINE_NAME),
	],
	min_members: 2,
	preserve_source_order: true,
};
const SPAN_GRID_LINE_VALUE: ValueGrammar = ValueGrammar::Or(&[
	POSITIVE_INTEGER,
	GRID_LINE_NAME,
	POSITIVE_GRID_LINE_NUMBER_AND_NAME,
]);
const SPAN_GRID_LINE: ValueGrammar = ValueGrammar::Ordered(&[
	required("span", &KW_SPAN),
	required("line", &SPAN_GRID_LINE_VALUE),
]);
const GRID_LINE: ValueGrammar = ValueGrammar::Or(&[
	KW_AUTO,
	I,
	GRID_LINE_NAME,
	GRID_LINE_NUMBER_AND_NAME,
	SPAN_GRID_LINE,
]);
const TRACK: ValueGrammar = ValueGrammar::Or(&[NLP, NFR, KW_TRACK]);
const TRACK_LIST: ValueGrammar = ValueGrammar::Space {
	min: 1,
	max: None,
	item: &TRACK,
};
const FONT_FAMILY_IDENTIFIERS: ValueGrammar = ValueGrammar::Space {
	min: 1,
	max: None,
	item: &IDENT,
};
const FONT_FAMILY_ITEM: ValueGrammar = ValueGrammar::Or(&[S, FONT_FAMILY_IDENTIFIERS]);
const FONT_FAMILY: ValueGrammar = ValueGrammar::Comma {
	min: 1,
	item: &FONT_FAMILY_ITEM,
};
const TIMING: ValueGrammar = KW_TIMING;

const INSET_VALUE: ValueGrammar = ValueGrammar::Or(&[LP, KW_AUTO]);
const INSET: ValueGrammar = ValueGrammar::Space {
	min: 1,
	max: Some(4),
	item: &INSET_VALUE,
};
const OVERFLOW: ValueGrammar = ValueGrammar::Space {
	min: 1,
	max: Some(2),
	item: &KW_OVERFLOW,
};
const Z_INDEX: ValueGrammar = ValueGrammar::Or(&[KW_AUTO, I]);
const MARGIN: ValueGrammar = ValueGrammar::Space {
	min: 1,
	max: Some(4),
	item: &INSET_VALUE,
};
const PADDING: ValueGrammar = ValueGrammar::Space {
	min: 1,
	max: Some(4),
	item: &NLP,
};
const FLEX_ORDERED: ValueGrammar = ValueGrammar::Ordered(&[
	required("grow", &NN),
	optional("shrink", &NN),
	optional("basis", &FLEX_BASIS),
]);
const FLEX_BASIS: ValueGrammar = ValueGrammar::Or(&[SIZE, KW_FLEX_BASIS]);
const FLEX: ValueGrammar = ValueGrammar::Or(&[KW_FLEX, FLEX_BASIS, FLEX_ORDERED]);
const FLEX_FLOW: ValueGrammar = ValueGrammar::Unordered {
	members: &[
		optional("direction", &KW_FLEX_DIRECTION),
		optional("wrap", &KW_FLEX_WRAP),
	],
	min_members: 1,
	preserve_source_order: false,
};
const GAP: ValueGrammar = ValueGrammar::Or(&[
	KW_NORMAL,
	ValueGrammar::Space {
		min: 1,
		max: Some(2),
		item: &NLP,
	},
]);
const SINGLE_GAP: ValueGrammar = ValueGrammar::Or(&[KW_NORMAL, NLP]);
const PLACE_CONTENT: ValueGrammar = ValueGrammar::Ordered(&[
	required("align", &KW_ALIGN_CONTENT),
	optional("justify", &KW_JUSTIFY_CONTENT),
]);
const PLACE_ITEMS: ValueGrammar = ValueGrammar::Ordered(&[
	required("align", &KW_ALIGN_ITEMS),
	optional("justify", &KW_JUSTIFY_ITEMS),
]);
const PLACE_SELF: ValueGrammar = ValueGrammar::Ordered(&[
	required("align", &KW_ALIGN_SELF),
	optional("justify", &KW_JUSTIFY_SELF),
]);
const OPTIONAL_TRACK_LIST: ValueGrammar = ValueGrammar::Or(&[KW_NONE, TRACK_LIST]);
const GRID_AREAS: ValueGrammar = ValueGrammar::Or(&[
	KW_NONE,
	ValueGrammar::Space {
		min: 1,
		max: None,
		item: &S,
	},
]);
const GRID_AUTO_FLOW: ValueGrammar = ValueGrammar::Unordered {
	members: &[
		optional("axis", &KW_GRID_FLOW_AXIS),
		optional("density", &KW_DENSE),
	],
	min_members: 1,
	preserve_source_order: false,
};
const GRID_LINE_PAIR: ValueGrammar = ValueGrammar::Slash {
	left: &GRID_LINE,
	right: &GRID_LINE,
};
const GRID_ROW_OR_COLUMN: ValueGrammar = ValueGrammar::Or(&[GRID_LINE, GRID_LINE_PAIR]);
const GRID_AREA: ValueGrammar = ValueGrammar::SlashList {
	min: 1,
	max: 4,
	item: &GRID_LINE,
};
const GRID_TRACK_PAIR: ValueGrammar = ValueGrammar::Slash {
	left: &TRACK_LIST,
	right: &TRACK_LIST,
};
const GRID_TEMPLATE: ValueGrammar = ValueGrammar::Or(&[KW_NONE, GRID_TRACK_PAIR]);

const FONT_SIZE: ValueGrammar = ValueGrammar::Or(&[NLP, KW_FONT_SIZE]);
const OBLIQUE_ANGLE: ValueGrammar = A;
const FONT_STYLE_OBLIQUE: ValueGrammar = ValueGrammar::Ordered(&[
	required("oblique", &KW_OBLIQUE),
	optional("angle", &OBLIQUE_ANGLE),
]);
const FONT_STYLE: ValueGrammar = ValueGrammar::Or(&[KW_FONT_STYLE, FONT_STYLE_OBLIQUE]);
const FONT_WEIGHT: ValueGrammar = ValueGrammar::Or(&[FONT_WEIGHT_NUMBER, KW_FONT_WEIGHT]);
const LINE_HEIGHT: ValueGrammar = ValueGrammar::Or(&[KW_NORMAL, NN, NLP]);
const LETTER_SPACING: ValueGrammar = ValueGrammar::Or(&[KW_NORMAL, L]);
const FONT_PREFIX: ValueGrammar = ValueGrammar::Unordered {
	members: &[
		optional("style", &FONT_STYLE),
		optional("variant", &KW_FONT_VARIANT),
		optional("weight", &FONT_WEIGHT),
	],
	min_members: 0,
	preserve_source_order: true,
};
const FONT_WITHOUT_LINE_HEIGHT: ValueGrammar = ValueGrammar::Ordered(&[
	required("prefix", &FONT_PREFIX),
	required("size", &FONT_SIZE),
	required("family", &FONT_FAMILY),
]);
const FONT_SIZE_AND_LINE_HEIGHT: ValueGrammar = ValueGrammar::Slash {
	left: &FONT_SIZE,
	right: &LINE_HEIGHT,
};
const FONT_WITH_LINE_HEIGHT: ValueGrammar = ValueGrammar::Ordered(&[
	required("prefix", &FONT_PREFIX),
	required("size-and-line-height", &FONT_SIZE_AND_LINE_HEIGHT),
	required("family", &FONT_FAMILY),
]);
const FONT: ValueGrammar = ValueGrammar::Or(&[
	KW_SYSTEM_FONT,
	FONT_WITHOUT_LINE_HEIGHT,
	FONT_WITH_LINE_HEIGHT,
]);
const TEXT_OVERFLOW_ITEM: ValueGrammar = ValueGrammar::Or(&[KW_TEXT_OVERFLOW, S]);
const TEXT_OVERFLOW: ValueGrammar = ValueGrammar::Space {
	min: 1,
	max: Some(2),
	item: &TEXT_OVERFLOW_ITEM,
};
const TEXT_DECORATION_LINES: ValueGrammar = ValueGrammar::Space {
	min: 1,
	max: Some(3),
	item: &KW_TEXT_DECORATION_LINE,
};
const TEXT_DECORATION_THICKNESS: ValueGrammar =
	ValueGrammar::Or(&[KW_TEXT_DECORATION_THICKNESS, NLP]);
const TEXT_DECORATION_BODY: ValueGrammar = ValueGrammar::Unordered {
	members: &[
		optional("line", &TEXT_DECORATION_LINES),
		optional("style", &KW_TEXT_DECORATION_STYLE),
		optional("color", &C),
		optional("thickness", &TEXT_DECORATION_THICKNESS),
	],
	min_members: 1,
	preserve_source_order: false,
};
const TEXT_DECORATION: ValueGrammar = ValueGrammar::Or(&[KW_NONE, TEXT_DECORATION_BODY]);

const BACKGROUND_IMAGE_LAYER: ValueGrammar = ValueGrammar::Or(&[KW_NONE, IMG]);
const BACKGROUND_IMAGE: ValueGrammar = ValueGrammar::Comma {
	min: 1,
	item: &BACKGROUND_IMAGE_LAYER,
};
const BACKGROUND_POSITION: ValueGrammar = ValueGrammar::Comma {
	min: 1,
	item: &POSITION,
};
const BACKGROUND_REPEAT_PAIR: ValueGrammar = ValueGrammar::Space {
	min: 2,
	max: Some(2),
	item: &KW_BACKGROUND_REPEAT_PAIR,
};
const BACKGROUND_REPEAT_LAYER: ValueGrammar = ValueGrammar::Or(&[
	KW_BACKGROUND_REPEAT,
	KW_BACKGROUND_REPEAT_PAIR,
	BACKGROUND_REPEAT_PAIR,
]);
const BACKGROUND_REPEAT: ValueGrammar = ValueGrammar::Comma {
	min: 1,
	item: &BACKGROUND_REPEAT_LAYER,
};
const BACKGROUND_SIZE_VALUES: ValueGrammar = ValueGrammar::Space {
	min: 1,
	max: Some(2),
	item: &BACKGROUND_SIZE_VALUE,
};
const BACKGROUND_SIZE_VALUE: ValueGrammar = ValueGrammar::Or(&[KW_AUTO, NLP]);
const BACKGROUND_SIZE_LAYER: ValueGrammar =
	ValueGrammar::Or(&[KW_BACKGROUND_SIZE, BACKGROUND_SIZE_VALUES]);
const BACKGROUND_SIZE: ValueGrammar = ValueGrammar::Comma {
	min: 1,
	item: &BACKGROUND_SIZE_LAYER,
};
const BACKGROUND_POSITION_SIZE: ValueGrammar = ValueGrammar::Slash {
	left: &POSITION,
	right: &BACKGROUND_SIZE_LAYER,
};
const BACKGROUND_LAYER: ValueGrammar = ValueGrammar::Unordered {
	members: &[
		optional("image", &BACKGROUND_IMAGE_LAYER),
		optional("position", &POSITION),
		optional("repeat", &BACKGROUND_REPEAT_LAYER),
		optional("position-size", &BACKGROUND_POSITION_SIZE),
	],
	min_members: 1,
	preserve_source_order: false,
};
const BACKGROUND_FINAL_LAYER: ValueGrammar = ValueGrammar::Unordered {
	members: &[
		optional("color", &C),
		optional("image", &BACKGROUND_IMAGE_LAYER),
		optional("position", &POSITION),
		optional("repeat", &BACKGROUND_REPEAT_LAYER),
		optional("position-size", &BACKGROUND_POSITION_SIZE),
	],
	min_members: 1,
	preserve_source_order: false,
};
const BACKGROUND: ValueGrammar = ValueGrammar::CommaFinal {
	min: 1,
	item: &BACKGROUND_LAYER,
	final_item: &BACKGROUND_FINAL_LAYER,
};
const BORDER_WIDTH: ValueGrammar = ValueGrammar::Space {
	min: 1,
	max: Some(4),
	item: &LINE_WIDTH,
};
const BORDER_STYLE: ValueGrammar = ValueGrammar::Space {
	min: 1,
	max: Some(4),
	item: &LINE_STYLE,
};
const BORDER_COLOR: ValueGrammar = ValueGrammar::Space {
	min: 1,
	max: Some(4),
	item: &C,
};
const RADIUS_VALUES: ValueGrammar = ValueGrammar::Space {
	min: 1,
	max: Some(4),
	item: &NLP,
};
const BORDER_RADIUS_SLASH: ValueGrammar = ValueGrammar::Slash {
	left: &RADIUS_VALUES,
	right: &RADIUS_VALUES,
};
const BORDER_RADIUS: ValueGrammar = ValueGrammar::Or(&[RADIUS_VALUES, BORDER_RADIUS_SLASH]);
const CORNER_RADIUS: ValueGrammar = ValueGrammar::Space {
	min: 1,
	max: Some(2),
	item: &NLP,
};

const SHADOW: ValueGrammar = ValueGrammar::Unordered {
	members: &[
		optional("inset", &KW_INSET),
		required("offset-x", &L),
		required("offset-y", &L),
		optional("blur", &NL),
		optional("spread", &L),
		optional("color", &C),
	],
	min_members: 2,
	preserve_source_order: true,
};
const SHADOW_LIST: ValueGrammar = ValueGrammar::Comma {
	min: 1,
	item: &SHADOW,
};
const BOX_SHADOW: ValueGrammar = ValueGrammar::Or(&[KW_NONE, SHADOW_LIST]);
const OPACITY: ValueGrammar = ValueGrammar::Or(&[N, P]);
const OUTLINE_COLOR: ValueGrammar = ValueGrammar::Or(&[C, KW_INVERT]);
const OUTLINE: ValueGrammar = ValueGrammar::Unordered {
	members: &[
		optional("width", &LINE_WIDTH),
		optional("style", &KW_OUTLINE_STYLE),
		optional("color", &OUTLINE_COLOR),
	],
	min_members: 1,
	preserve_source_order: false,
};

const TRANSFORM: ValueGrammar = ValueGrammar::Or(&[
	KW_NONE,
	ValueGrammar::Space {
		min: 1,
		max: None,
		item: &TF,
	},
]);
const TRANSFORM_ORIGIN: ValueGrammar = ValueGrammar::Ordered(&[
	required("position", &TRANSFORM_ORIGIN_POSITION),
	optional("z-offset", &L),
]);
const TRANSITION_PROPERTY: ValueGrammar = ValueGrammar::Or(&[KW_TRANSITION_PROPERTY, IDENT]);
const TRANSITION_PROPERTY_LIST_IDENTIFIER: ValueGrammar = ValueGrammar::IdentifierExcept(&["none"]);
const TRANSITION_PROPERTY_LIST_ITEM: ValueGrammar = ValueGrammar::Or(&[
	KW_TRANSITION_PROPERTY_LIST,
	TRANSITION_PROPERTY_LIST_IDENTIFIER,
]);
const TRANSITION_PROPERTY_LIST: ValueGrammar = ValueGrammar::Or(&[
	KW_NONE,
	ValueGrammar::Comma {
		min: 1,
		item: &TRANSITION_PROPERTY_LIST_ITEM,
	},
]);
const TIME_LIST: ValueGrammar = ValueGrammar::Comma { min: 1, item: &T };
const DURATION_LIST: ValueGrammar = ValueGrammar::Comma { min: 1, item: &NT };
const TIMING_LIST: ValueGrammar = ValueGrammar::Comma {
	min: 1,
	item: &TIMING,
};
const TRANSITION_LAYER: ValueGrammar = ValueGrammar::Unordered {
	members: &[
		optional("property", &TRANSITION_PROPERTY),
		optional("duration", &NT),
		optional("timing-function", &TIMING),
		optional("delay", &T),
	],
	min_members: 1,
	preserve_source_order: true,
};
const TRANSITION: ValueGrammar = ValueGrammar::Comma {
	min: 1,
	item: &TRANSITION_LAYER,
};

const TOUCH_GESTURES: ValueGrammar = ValueGrammar::Unordered {
	members: &[
		optional("x", &KW_TOUCH_X_GESTURE),
		optional("y", &KW_TOUCH_Y_GESTURE),
		optional("pinch-zoom", &KW_TOUCH_PINCH_ZOOM),
	],
	min_members: 1,
	preserve_source_order: true,
};
const TOUCH_ACTION: ValueGrammar = ValueGrammar::Or(&[KW_TOUCH_ACTION, TOUCH_GESTURES]);
const CONTENT: ValueGrammar = ValueGrammar::Or(&[KW_CONTENT, S]);
const LIST_STYLE_TYPE: ValueGrammar = ValueGrammar::Or(&[KW_LIST_TYPE, IDENT]);
const LIST_STYLE: ValueGrammar = ValueGrammar::Unordered {
	members: &[
		optional("position", &KW_LIST_POSITION),
		optional("type", &LIST_STYLE_TYPE),
		optional("none", &KW_NONE),
	],
	min_members: 1,
	preserve_source_order: false,
};

const fn property(
	name: &'static str,
	family: PropertyFamily,
	grammar: &'static ValueGrammar,
) -> PropertySpec {
	PropertySpec {
		name,
		family,
		grammar,
		css_wide_keywords: &CSS_WIDE_DOMAIN,
	}
}

macro_rules! property_registry {
	($($family:expr => { $($name:literal => $grammar:ident),+ $(,)? }),+ $(,)?) => {
		&[$($(property($name, $family, &$grammar)),+),+]
	};
}

static PROPERTY_SPECS: &[PropertySpec] = property_registry!(
	PropertyFamily::Layout => {
		"display" => KW_DISPLAY,
		"position" => KW_POSITION_MODE,
		"inset" => INSET,
		"top" => INSET_VALUE,
		"right" => INSET_VALUE,
		"bottom" => INSET_VALUE,
		"left" => INSET_VALUE,
		"float" => KW_FLOAT,
		"clear" => KW_CLEAR,
		"overflow" => OVERFLOW,
		"overflow-x" => KW_OVERFLOW,
		"overflow-y" => KW_OVERFLOW,
		"visibility" => KW_VISIBILITY,
		"z-index" => Z_INDEX,
	},
	PropertyFamily::BoxModel => {
		"box-sizing" => KW_BOX_SIZING,
		"width" => SIZE,
		"min-width" => SIZE,
		"max-width" => MAX_SIZE,
		"height" => SIZE,
		"min-height" => SIZE,
		"max-height" => MAX_SIZE,
		"margin" => MARGIN,
		"margin-top" => INSET_VALUE,
		"margin-right" => INSET_VALUE,
		"margin-bottom" => INSET_VALUE,
		"margin-left" => INSET_VALUE,
		"padding" => PADDING,
		"padding-top" => NLP,
		"padding-right" => NLP,
		"padding-bottom" => NLP,
		"padding-left" => NLP,
	},
	PropertyFamily::FlexAndGrid => {
		"flex" => FLEX,
		"flex-basis" => FLEX_BASIS,
		"flex-grow" => NN,
		"flex-shrink" => NN,
		"order" => I,
		"flex-direction" => KW_FLEX_DIRECTION,
		"flex-wrap" => KW_FLEX_WRAP,
		"flex-flow" => FLEX_FLOW,
		"gap" => GAP,
		"row-gap" => SINGLE_GAP,
		"column-gap" => SINGLE_GAP,
		"align-content" => KW_ALIGN_CONTENT,
		"align-items" => KW_ALIGN_ITEMS,
		"align-self" => KW_ALIGN_SELF,
		"justify-content" => KW_JUSTIFY_CONTENT,
		"justify-items" => KW_JUSTIFY_ITEMS,
		"justify-self" => KW_JUSTIFY_SELF,
		"place-content" => PLACE_CONTENT,
		"place-items" => PLACE_ITEMS,
		"place-self" => PLACE_SELF,
		"grid-template-columns" => OPTIONAL_TRACK_LIST,
		"grid-template-rows" => OPTIONAL_TRACK_LIST,
		"grid-auto-columns" => TRACK_LIST,
		"grid-auto-rows" => TRACK_LIST,
		"grid-template-areas" => GRID_AREAS,
		"grid-auto-flow" => GRID_AUTO_FLOW,
		"grid-column" => GRID_ROW_OR_COLUMN,
		"grid-row" => GRID_ROW_OR_COLUMN,
		"grid-area" => GRID_AREA,
		"grid-template" => GRID_TEMPLATE,
		"grid" => GRID_TEMPLATE,
	},
	PropertyFamily::Typography => {
		"color" => C,
		"font-family" => FONT_FAMILY,
		"font-size" => FONT_SIZE,
		"font-style" => FONT_STYLE,
		"font-variant" => KW_FONT_VARIANT,
		"font-weight" => FONT_WEIGHT,
		"line-height" => LINE_HEIGHT,
		"letter-spacing" => LETTER_SPACING,
		"font" => FONT,
		"text-align" => KW_TEXT_ALIGN,
		"text-overflow" => TEXT_OVERFLOW,
		"text-decoration" => TEXT_DECORATION,
		"text-transform" => KW_TEXT_TRANSFORM,
		"text-wrap" => KW_TEXT_WRAP,
		"white-space" => KW_WHITE_SPACE,
		"word-break" => KW_WORD_BREAK,
	},
	PropertyFamily::BackgroundAndBorder => {
		"background-color" => C,
		"background-image" => BACKGROUND_IMAGE,
		"background-position" => BACKGROUND_POSITION,
		"background-repeat" => BACKGROUND_REPEAT,
		"background-size" => BACKGROUND_SIZE,
		"background" => BACKGROUND,
		"border" => BORDER,
		"border-top" => BORDER,
		"border-right" => BORDER,
		"border-bottom" => BORDER,
		"border-left" => BORDER,
		"border-width" => BORDER_WIDTH,
		"border-top-width" => LINE_WIDTH,
		"border-right-width" => LINE_WIDTH,
		"border-bottom-width" => LINE_WIDTH,
		"border-left-width" => LINE_WIDTH,
		"border-style" => BORDER_STYLE,
		"border-top-style" => LINE_STYLE,
		"border-right-style" => LINE_STYLE,
		"border-bottom-style" => LINE_STYLE,
		"border-left-style" => LINE_STYLE,
		"border-color" => BORDER_COLOR,
		"border-top-color" => C,
		"border-right-color" => C,
		"border-bottom-color" => C,
		"border-left-color" => C,
		"border-radius" => BORDER_RADIUS,
		"border-top-left-radius" => CORNER_RADIUS,
		"border-top-right-radius" => CORNER_RADIUS,
		"border-bottom-right-radius" => CORNER_RADIUS,
		"border-bottom-left-radius" => CORNER_RADIUS,
	},
	PropertyFamily::Effects => {
		"box-shadow" => BOX_SHADOW,
		"opacity" => OPACITY,
		"outline" => OUTLINE,
		"outline-color" => OUTLINE_COLOR,
		"outline-offset" => L,
		"outline-style" => KW_OUTLINE_STYLE,
		"outline-width" => LINE_WIDTH,
		"filter" => KW_NONE,
		"backdrop-filter" => KW_NONE,
	},
	PropertyFamily::TransformAndTransition => {
		"transform" => TRANSFORM,
		"transform-origin" => TRANSFORM_ORIGIN,
		"transition-property" => TRANSITION_PROPERTY_LIST,
		"transition-duration" => DURATION_LIST,
		"transition-timing-function" => TIMING_LIST,
		"transition-delay" => TIME_LIST,
		"transition" => TRANSITION,
	},
	PropertyFamily::InteractionAndGenerated => {
		"cursor" => KW_CURSOR,
		"pointer-events" => KW_POINTER_EVENTS,
		"resize" => KW_RESIZE,
		"touch-action" => TOUCH_ACTION,
		"user-select" => KW_USER_SELECT,
		"content" => CONTENT,
		"list-style-position" => KW_LIST_POSITION,
		"list-style-type" => LIST_STYLE_TYPE,
		"list-style" => LIST_STYLE,
	},
);

/// Returns the complete immutable MVP property registry.
pub fn property_specs() -> &'static [PropertySpec] {
	PROPERTY_SPECS
}

// This crate-private lookup is the validation boundary for parsed declarations.
#[allow(dead_code)]
pub(crate) fn property_spec(name: &str) -> Option<&'static PropertySpec> {
	PROPERTY_SPECS.iter().find(|spec| spec.name == name)
}

/// The accepted number of arguments for a registered function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArityPolicy {
	/// Exactly this many arguments are required.
	Exact(usize),
	/// At least this many arguments are required.
	AtLeast(usize),
}

impl ArityPolicy {
	const fn accepts(self, count: usize) -> bool {
		match self {
			Self::Exact(expected) => count == expected,
			Self::AtLeast(minimum) => count >= minimum,
		}
	}

	fn describe(self) -> String {
		match self {
			Self::Exact(count) => format!("exact:{count}"),
			Self::AtLeast(count) => format!("at-least:{count}"),
		}
	}
}

/// How argument constraints apply across a function's arity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArgumentConstraints {
	/// One constraint applies to every argument.
	Repeated(TypeConstraint),
	/// Each argument has its own positional constraint.
	Positional(&'static [TypeConstraint]),
}

impl ArgumentConstraints {
	fn describe(self) -> String {
		match self {
			Self::Repeated(constraint) => {
				format!("repeated:{}", describe_constraint(constraint))
			}
			Self::Positional(constraints) => format!(
				"positional:[{}]",
				constraints
					.iter()
					.map(|constraint| describe_constraint(*constraint))
					.collect::<Vec<_>>()
					.join(",")
			),
		}
	}
}

/// The result relationship of a registered function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FunctionResult {
	/// The function always produces one exact semantic type.
	Exact(SemanticType),
	/// The result is the numeric join of all participating arguments.
	JoinedNumeric,
	/// The result retains both generic operand types as a slash pair.
	SlashPair,
}

impl FunctionResult {
	fn describe(self) -> &'static str {
		match self {
			Self::Exact(semantic_type) => semantic_type_name(semantic_type),
			Self::JoinedNumeric => "JOINED_NUMERIC",
			Self::SlashPair => "SLASH_PAIR",
		}
	}
}

/// How already-rendered fragments are assembled into CSS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LoweringStrategy {
	/// A CSS function with comma-separated arguments.
	CommaFunction,
	/// A CSS function with modern space-separated arguments.
	SpaceFunction,
	/// The fixed sRGB `color-mix` receiver lowering.
	ColorMixSrgb,
	/// Two fragments separated by one space without a wrapper function.
	SpacePair,
	/// Two fragments separated by a slash without a wrapper function.
	SlashPair,
}

impl LoweringStrategy {
	const fn describe(self) -> &'static str {
		match self {
			Self::CommaFunction => "comma-function",
			Self::SpaceFunction => "space-function",
			Self::ColorMixSrgb => "color-mix-srgb",
			Self::SpacePair => "space-pair",
			Self::SlashPair => "slash-pair",
		}
	}
}

/// One immutable typed function-registry entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FunctionSpec {
	/// The exact DSL path or receiver-method spelling.
	pub dsl_path: &'static str,
	/// The emitted CSS function or structural spelling.
	pub css_spelling: &'static str,
	/// The accepted argument count.
	pub arity: ArityPolicy,
	/// The data-driven argument constraints.
	pub arguments: ArgumentConstraints,
	/// The receiver constraint for a method, if any.
	pub receiver: Option<TypeConstraint>,
	/// The result type relationship.
	pub result: FunctionResult,
	/// The fragment lowering strategy.
	pub lowering: LoweringStrategy,
}

impl FunctionSpec {
	/// Returns a stable description containing every normative field.
	pub fn describe(&self) -> String {
		format!(
			"{}|css={}|arity={}|args={}|receiver={}|result={}|lowering={}",
			self.dsl_path,
			self.css_spelling,
			self.arity.describe(),
			self.arguments.describe(),
			self.receiver
				.map_or_else(|| "none".to_owned(), describe_constraint),
			self.result.describe(),
			self.lowering.describe(),
		)
	}

	/// Lowers already-rendered, type-checked fragments according to this spec.
	// This internal boundary is consumed only after semantic validation succeeds.
	#[allow(dead_code)]
	pub(crate) fn lower_rendered(
		&self,
		receiver: Option<&str>,
		arguments: &[&str],
	) -> Option<String> {
		if !self.arity.accepts(arguments.len()) || self.receiver.is_some() != receiver.is_some() {
			return None;
		}

		match self.lowering {
			LoweringStrategy::CommaFunction => {
				Some(format!("{}({})", self.css_spelling, arguments.join(", ")))
			}
			LoweringStrategy::SpaceFunction => {
				Some(format!("{}({})", self.css_spelling, arguments.join(" ")))
			}
			LoweringStrategy::ColorMixSrgb => {
				let receiver = receiver?;
				let [other, amount] = arguments else {
					return None;
				};
				Some(format!(
					"{}(in srgb, {receiver} calc(100% - {amount}), {other} {amount})",
					self.css_spelling
				))
			}
			LoweringStrategy::SpacePair => {
				let [left, right] = arguments else {
					return None;
				};
				Some(format!("{left} {right}"))
			}
			LoweringStrategy::SlashPair => {
				let [left, right] = arguments else {
					return None;
				};
				Some(format!("{left} / {right}"))
			}
		}
	}
}

fn describe_constraint(constraint: TypeConstraint) -> String {
	match constraint {
		TypeConstraint::Exact(semantic_type) => semantic_type_name(semantic_type).to_owned(),
		TypeConstraint::Numeric(NumericConstraint::NumberOrPercentage) => {
			"NUMERIC(NUMBER_OR_PERCENTAGE)".to_owned()
		}
		TypeConstraint::Numeric(NumericConstraint::PercentageRange { minimum, maximum }) => {
			format!("NUMERIC(PERCENTAGE_RANGE({minimum},{maximum}))")
		}
		TypeConstraint::Numeric(NumericConstraint::Joined) => "NUMERIC(JOINED)".to_owned(),
		TypeConstraint::CommaList { element, min } => {
			format!("COMMA_LIST(min={min},{})", semantic_type_name(element))
		}
		TypeConstraint::Any => "ANY".to_owned(),
	}
}

const JOINED: TypeConstraint = TypeConstraint::Numeric(NumericConstraint::Joined);
const NUMBER_OR_PERCENTAGE: TypeConstraint =
	TypeConstraint::Numeric(NumericConstraint::NumberOrPercentage);
const MIX_PERCENTAGE: TypeConstraint =
	TypeConstraint::Numeric(NumericConstraint::PercentageRange {
		minimum: 0,
		maximum: 100,
	});

const fn function(
	dsl_path: &'static str,
	css_spelling: &'static str,
	arity: ArityPolicy,
	arguments: ArgumentConstraints,
	receiver: Option<TypeConstraint>,
	result: FunctionResult,
	lowering: LoweringStrategy,
) -> FunctionSpec {
	FunctionSpec {
		dsl_path,
		css_spelling,
		arity,
		arguments,
		receiver,
		result,
		lowering,
	}
}

static FUNCTION_SPECS: &[FunctionSpec] = &[
	function(
		"min",
		"min",
		ArityPolicy::AtLeast(2),
		ArgumentConstraints::Repeated(JOINED),
		None,
		FunctionResult::JoinedNumeric,
		LoweringStrategy::CommaFunction,
	),
	function(
		"max",
		"max",
		ArityPolicy::AtLeast(2),
		ArgumentConstraints::Repeated(JOINED),
		None,
		FunctionResult::JoinedNumeric,
		LoweringStrategy::CommaFunction,
	),
	function(
		"clamp",
		"clamp",
		ArityPolicy::Exact(3),
		ArgumentConstraints::Positional(&[JOINED, JOINED, JOINED]),
		None,
		FunctionResult::JoinedNumeric,
		LoweringStrategy::CommaFunction,
	),
	function(
		"Color::rgb",
		"rgb",
		ArityPolicy::Exact(3),
		ArgumentConstraints::Positional(&[
			NUMBER_OR_PERCENTAGE,
			NUMBER_OR_PERCENTAGE,
			NUMBER_OR_PERCENTAGE,
		]),
		None,
		FunctionResult::Exact(SemanticType::Color),
		LoweringStrategy::SpaceFunction,
	),
	function(
		"Color::hsl",
		"hsl",
		ArityPolicy::Exact(3),
		ArgumentConstraints::Positional(&[
			TypeConstraint::Exact(SemanticType::Angle),
			TypeConstraint::Exact(SemanticType::Percentage),
			TypeConstraint::Exact(SemanticType::Percentage),
		]),
		None,
		FunctionResult::Exact(SemanticType::Color),
		LoweringStrategy::SpaceFunction,
	),
	function(
		"Color::oklch",
		"oklch",
		ArityPolicy::Exact(3),
		ArgumentConstraints::Positional(&[
			NUMBER_OR_PERCENTAGE,
			NUMBER_OR_PERCENTAGE,
			TypeConstraint::Exact(SemanticType::Angle),
		]),
		None,
		FunctionResult::Exact(SemanticType::Color),
		LoweringStrategy::SpaceFunction,
	),
	function(
		".mix",
		"color-mix",
		ArityPolicy::Exact(2),
		ArgumentConstraints::Positional(&[
			TypeConstraint::Exact(SemanticType::Color),
			MIX_PERCENTAGE,
		]),
		Some(TypeConstraint::Exact(SemanticType::Color)),
		FunctionResult::Exact(SemanticType::Color),
		LoweringStrategy::ColorMixSrgb,
	),
	function(
		"stop",
		"<color> <position>",
		ArityPolicy::Exact(2),
		ArgumentConstraints::Positional(&[
			TypeConstraint::Exact(SemanticType::Color),
			TypeConstraint::Exact(SemanticType::LengthPercentage),
		]),
		None,
		FunctionResult::Exact(SemanticType::GradientStop),
		LoweringStrategy::SpacePair,
	),
	function(
		"linear_gradient",
		"linear-gradient",
		ArityPolicy::Exact(2),
		ArgumentConstraints::Positional(&[
			TypeConstraint::Exact(SemanticType::Direction),
			TypeConstraint::CommaList {
				element: SemanticType::GradientStop,
				min: 2,
			},
		]),
		None,
		FunctionResult::Exact(SemanticType::Image),
		LoweringStrategy::CommaFunction,
	),
	function(
		"translate",
		"translate",
		ArityPolicy::Exact(2),
		ArgumentConstraints::Positional(&[
			TypeConstraint::Exact(SemanticType::LengthPercentage),
			TypeConstraint::Exact(SemanticType::LengthPercentage),
		]),
		None,
		FunctionResult::Exact(SemanticType::TransformFunction),
		LoweringStrategy::CommaFunction,
	),
	function(
		"translate_x",
		"translateX",
		ArityPolicy::Exact(1),
		ArgumentConstraints::Positional(&[TypeConstraint::Exact(SemanticType::LengthPercentage)]),
		None,
		FunctionResult::Exact(SemanticType::TransformFunction),
		LoweringStrategy::CommaFunction,
	),
	function(
		"translate_y",
		"translateY",
		ArityPolicy::Exact(1),
		ArgumentConstraints::Positional(&[TypeConstraint::Exact(SemanticType::LengthPercentage)]),
		None,
		FunctionResult::Exact(SemanticType::TransformFunction),
		LoweringStrategy::CommaFunction,
	),
	function(
		"rotate",
		"rotate",
		ArityPolicy::Exact(1),
		ArgumentConstraints::Positional(&[TypeConstraint::Exact(SemanticType::Angle)]),
		None,
		FunctionResult::Exact(SemanticType::TransformFunction),
		LoweringStrategy::CommaFunction,
	),
	function(
		"scale",
		"scale",
		ArityPolicy::Exact(1),
		ArgumentConstraints::Positional(&[TypeConstraint::Exact(SemanticType::Number)]),
		None,
		FunctionResult::Exact(SemanticType::TransformFunction),
		LoweringStrategy::CommaFunction,
	),
	function(
		"scale_x",
		"scaleX",
		ArityPolicy::Exact(1),
		ArgumentConstraints::Positional(&[TypeConstraint::Exact(SemanticType::Number)]),
		None,
		FunctionResult::Exact(SemanticType::TransformFunction),
		LoweringStrategy::CommaFunction,
	),
	function(
		"scale_y",
		"scaleY",
		ArityPolicy::Exact(1),
		ArgumentConstraints::Positional(&[TypeConstraint::Exact(SemanticType::Number)]),
		None,
		FunctionResult::Exact(SemanticType::TransformFunction),
		LoweringStrategy::CommaFunction,
	),
	function(
		"slash",
		"<left> / <right>",
		ArityPolicy::Exact(2),
		ArgumentConstraints::Positional(&[TypeConstraint::Any, TypeConstraint::Any]),
		None,
		FunctionResult::SlashPair,
		LoweringStrategy::SlashPair,
	),
];

/// Returns the complete immutable initial function registry.
pub fn function_specs() -> &'static [FunctionSpec] {
	FUNCTION_SPECS
}

// This crate-private lookup is the validation boundary for parsed style calls.
#[allow(dead_code)]
pub(crate) fn function_spec(dsl_path: &str) -> Option<&'static FunctionSpec> {
	FUNCTION_SPECS.iter().find(|spec| spec.dsl_path == dsl_path)
}

/// A direct CSS function with dedicated rewrite guidance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReservedFunction {
	/// Direct custom-property lookup.
	Var,
	/// Direct CSS arithmetic wrapping.
	Calc,
}

impl ReservedFunction {
	/// Returns the dedicated diagnostic kind for this reserved call.
	pub const fn diagnostic_kind(self) -> StyleDiagnosticKind {
		match self {
			Self::Var => StyleDiagnosticKind::DirectVarCall,
			Self::Calc => StyleDiagnosticKind::DirectCalcCall,
		}
	}
}

// Reserved calls share the internal lookup path with registered style functions.
#[allow(dead_code)]
pub(crate) fn reserved_function(name: &str) -> Option<ReservedFunction> {
	match name {
		"var" => Some(ReservedFunction::Var),
		"calc" => Some(ReservedFunction::Calc),
		_ => None,
	}
}

/// Generates the public normative registry reference from the live tables.
pub fn registry_reference_text() -> String {
	let mut reference = String::from("Reinhardt checked style registry\n\n[properties]\n");
	for spec in property_specs() {
		writeln!(
			&mut reference,
			"property\t{}\t{:?}\t{}\tcss-wide=[{}]",
			spec.name,
			spec.family,
			spec.grammar.describe_with_keywords(),
			spec.css_wide_keywords.keywords.join("|")
		)
		.expect("writing registry data to a String cannot fail");
	}

	reference.push_str("\n[named-colors]\n");
	let named_colors = named_color_domain();
	for keyword in named_colors.keywords {
		writeln!(
			&mut reference,
			"named-color\t{keyword}\t{}",
			semantic_type_name(named_colors.produced_type)
		)
		.expect("writing registry data to a String cannot fail");
	}

	reference.push_str("\n[units]\n");
	for spec in unit_specs() {
		writeln!(
			&mut reference,
			"unit\t{}\t{:?}\t{:?}",
			spec.name, spec.dimension, spec.category
		)
		.expect("writing registry data to a String cannot fail");
	}

	reference.push_str("\n[functions]\n");
	for spec in function_specs() {
		writeln!(
			&mut reference,
			"function\t{}\t{}",
			spec.dsl_path,
			spec.describe()
		)
		.expect("writing registry data to a String cannot fail");
	}

	reference
}

#[cfg(test)]
mod tests {
	use std::collections::BTreeSet;

	use rstest::rstest;

	use super::{
		PropertyFamily, ReservedFunction, UnitCategory, ValueGrammar, function_spec,
		function_specs, infer_named_keyword_type, named_color_domain, property_spec,
		property_specs, registry_reference_text, reserved_function, unit_spec, unit_specs,
	};
	use crate::core::{NumericDimension, SemanticType};
	use crate::style::diagnostic::StyleDiagnosticKind;

	const EXPECTED_NAMED_COLORS: &[&str] = &[
		"aliceblue",
		"antiquewhite",
		"aqua",
		"aquamarine",
		"azure",
		"beige",
		"bisque",
		"black",
		"blanchedalmond",
		"blue",
		"blueviolet",
		"brown",
		"burlywood",
		"cadetblue",
		"chartreuse",
		"chocolate",
		"coral",
		"cornflowerblue",
		"cornsilk",
		"crimson",
		"cyan",
		"darkblue",
		"darkcyan",
		"darkgoldenrod",
		"darkgray",
		"darkgreen",
		"darkgrey",
		"darkkhaki",
		"darkmagenta",
		"darkolivegreen",
		"darkorange",
		"darkorchid",
		"darkred",
		"darksalmon",
		"darkseagreen",
		"darkslateblue",
		"darkslategray",
		"darkslategrey",
		"darkturquoise",
		"darkviolet",
		"deeppink",
		"deepskyblue",
		"dimgray",
		"dimgrey",
		"dodgerblue",
		"firebrick",
		"floralwhite",
		"forestgreen",
		"fuchsia",
		"gainsboro",
		"ghostwhite",
		"gold",
		"goldenrod",
		"gray",
		"green",
		"greenyellow",
		"grey",
		"honeydew",
		"hotpink",
		"indianred",
		"indigo",
		"ivory",
		"khaki",
		"lavender",
		"lavenderblush",
		"lawngreen",
		"lemonchiffon",
		"lightblue",
		"lightcoral",
		"lightcyan",
		"lightgoldenrodyellow",
		"lightgray",
		"lightgreen",
		"lightgrey",
		"lightpink",
		"lightsalmon",
		"lightseagreen",
		"lightskyblue",
		"lightslategray",
		"lightslategrey",
		"lightsteelblue",
		"lightyellow",
		"lime",
		"limegreen",
		"linen",
		"magenta",
		"maroon",
		"mediumaquamarine",
		"mediumblue",
		"mediumorchid",
		"mediumpurple",
		"mediumseagreen",
		"mediumslateblue",
		"mediumspringgreen",
		"mediumturquoise",
		"mediumvioletred",
		"midnightblue",
		"mintcream",
		"mistyrose",
		"moccasin",
		"navajowhite",
		"navy",
		"oldlace",
		"olive",
		"olivedrab",
		"orange",
		"orangered",
		"orchid",
		"palegoldenrod",
		"palegreen",
		"paleturquoise",
		"palevioletred",
		"papayawhip",
		"peachpuff",
		"peru",
		"pink",
		"plum",
		"powderblue",
		"purple",
		"rebeccapurple",
		"red",
		"rosybrown",
		"royalblue",
		"saddlebrown",
		"salmon",
		"sandybrown",
		"seagreen",
		"seashell",
		"sienna",
		"silver",
		"skyblue",
		"slateblue",
		"slategray",
		"slategrey",
		"snow",
		"springgreen",
		"steelblue",
		"tan",
		"teal",
		"thistle",
		"tomato",
		"transparent",
		"turquoise",
		"violet",
		"wheat",
		"white",
		"whitesmoke",
		"yellow",
		"yellowgreen",
		"currentcolor",
	];

	#[rstest]
	#[case(
		UnitCategory::AbsoluteLength,
		NumericDimension::Length,
		&["px", "cm", "mm", "q", "in", "pc", "pt"]
	)]
	#[case(
		UnitCategory::FontRelativeLength,
		NumericDimension::Length,
		&["em", "rem", "ex", "rex", "cap", "rcap", "ch", "rch", "ic", "ric", "lh", "rlh"]
	)]
	#[case(
		UnitCategory::ViewportLength,
		NumericDimension::Length,
		&[
			"vw", "vh", "vi", "vb", "vmin", "vmax", "svw", "svh", "svi", "svb",
			"svmin", "svmax", "lvw", "lvh", "lvi", "lvb", "lvmin", "lvmax", "dvw",
			"dvh", "dvi", "dvb", "dvmin", "dvmax",
		]
	)]
	#[case(
		UnitCategory::ContainerLength,
		NumericDimension::Length,
		&["cqw", "cqh", "cqi", "cqb", "cqmin", "cqmax"]
	)]
	#[case(
		UnitCategory::GridFraction,
		NumericDimension::GridFraction,
		&["fr"]
	)]
	#[case(
		UnitCategory::Angle,
		NumericDimension::Angle,
		&["deg", "grad", "rad", "turn"]
	)]
	#[case(UnitCategory::Time, NumericDimension::Time, &["ms", "s"])]
	#[case(
		UnitCategory::Percentage,
		NumericDimension::Percentage,
		&["%"]
	)]
	fn unit_registry_has_exact_group(
		#[case] category: UnitCategory,
		#[case] dimension: NumericDimension,
		#[case] expected_names: &[&str],
	) {
		// Arrange
		let specs = unit_specs();

		// Act
		let actual = specs
			.iter()
			.filter(|spec| spec.category == category)
			.map(|spec| spec.name)
			.collect::<Vec<_>>();
		let dimensions = specs
			.iter()
			.filter(|spec| spec.category == category)
			.map(|spec| spec.dimension)
			.collect::<Vec<_>>();

		// Assert
		assert_eq!(actual, expected_names);
		assert_eq!(dimensions, vec![dimension; expected_names.len()]);
	}

	#[rstest]
	fn unit_registry_names_are_unique_and_complete() {
		// Arrange
		let specs = unit_specs();

		// Act
		let names = specs.iter().map(|spec| spec.name).collect::<Vec<_>>();
		let unique = names.iter().copied().collect::<BTreeSet<_>>();

		// Assert
		assert_eq!(names.len(), 57);
		assert_eq!(unique.len(), names.len());
	}

	#[rstest]
	fn named_color_domain_has_exact_unique_production_set_and_color_result() {
		// Arrange
		let domain = named_color_domain();

		// Act
		let unique = domain.keywords.iter().copied().collect::<BTreeSet<_>>();

		// Assert
		assert_eq!(domain.name, "named-color");
		assert_eq!(domain.produced_type, SemanticType::Color);
		assert_eq!(domain.keywords, EXPECTED_NAMED_COLORS);
		assert_eq!(unique.len(), EXPECTED_NAMED_COLORS.len());
	}

	#[rstest]
	#[case("red", Some(SemanticType::Color))]
	#[case("black", Some(SemanticType::Color))]
	#[case("transparent", Some(SemanticType::Color))]
	#[case("currentcolor", Some(SemanticType::Color))]
	#[case("not-a-color", None)]
	fn named_keyword_inference_uses_canonical_color_domain(
		#[case] keyword: &str,
		#[case] expected: Option<SemanticType>,
	) {
		// Arrange and Act
		let actual = infer_named_keyword_type(keyword);

		// Assert
		assert_eq!(actual, expected);
	}

	#[rstest]
	fn property_registry_has_exact_names_and_families() {
		// Arrange
		let expected = [
			("display", PropertyFamily::Layout),
			("position", PropertyFamily::Layout),
			("inset", PropertyFamily::Layout),
			("top", PropertyFamily::Layout),
			("right", PropertyFamily::Layout),
			("bottom", PropertyFamily::Layout),
			("left", PropertyFamily::Layout),
			("float", PropertyFamily::Layout),
			("clear", PropertyFamily::Layout),
			("overflow", PropertyFamily::Layout),
			("overflow-x", PropertyFamily::Layout),
			("overflow-y", PropertyFamily::Layout),
			("visibility", PropertyFamily::Layout),
			("z-index", PropertyFamily::Layout),
			("box-sizing", PropertyFamily::BoxModel),
			("width", PropertyFamily::BoxModel),
			("min-width", PropertyFamily::BoxModel),
			("max-width", PropertyFamily::BoxModel),
			("height", PropertyFamily::BoxModel),
			("min-height", PropertyFamily::BoxModel),
			("max-height", PropertyFamily::BoxModel),
			("margin", PropertyFamily::BoxModel),
			("margin-top", PropertyFamily::BoxModel),
			("margin-right", PropertyFamily::BoxModel),
			("margin-bottom", PropertyFamily::BoxModel),
			("margin-left", PropertyFamily::BoxModel),
			("padding", PropertyFamily::BoxModel),
			("padding-top", PropertyFamily::BoxModel),
			("padding-right", PropertyFamily::BoxModel),
			("padding-bottom", PropertyFamily::BoxModel),
			("padding-left", PropertyFamily::BoxModel),
			("flex", PropertyFamily::FlexAndGrid),
			("flex-basis", PropertyFamily::FlexAndGrid),
			("flex-grow", PropertyFamily::FlexAndGrid),
			("flex-shrink", PropertyFamily::FlexAndGrid),
			("order", PropertyFamily::FlexAndGrid),
			("flex-direction", PropertyFamily::FlexAndGrid),
			("flex-wrap", PropertyFamily::FlexAndGrid),
			("flex-flow", PropertyFamily::FlexAndGrid),
			("gap", PropertyFamily::FlexAndGrid),
			("row-gap", PropertyFamily::FlexAndGrid),
			("column-gap", PropertyFamily::FlexAndGrid),
			("align-content", PropertyFamily::FlexAndGrid),
			("align-items", PropertyFamily::FlexAndGrid),
			("align-self", PropertyFamily::FlexAndGrid),
			("justify-content", PropertyFamily::FlexAndGrid),
			("justify-items", PropertyFamily::FlexAndGrid),
			("justify-self", PropertyFamily::FlexAndGrid),
			("place-content", PropertyFamily::FlexAndGrid),
			("place-items", PropertyFamily::FlexAndGrid),
			("place-self", PropertyFamily::FlexAndGrid),
			("grid-template-columns", PropertyFamily::FlexAndGrid),
			("grid-template-rows", PropertyFamily::FlexAndGrid),
			("grid-auto-columns", PropertyFamily::FlexAndGrid),
			("grid-auto-rows", PropertyFamily::FlexAndGrid),
			("grid-template-areas", PropertyFamily::FlexAndGrid),
			("grid-auto-flow", PropertyFamily::FlexAndGrid),
			("grid-column", PropertyFamily::FlexAndGrid),
			("grid-row", PropertyFamily::FlexAndGrid),
			("grid-area", PropertyFamily::FlexAndGrid),
			("grid-template", PropertyFamily::FlexAndGrid),
			("grid", PropertyFamily::FlexAndGrid),
			("color", PropertyFamily::Typography),
			("font-family", PropertyFamily::Typography),
			("font-size", PropertyFamily::Typography),
			("font-style", PropertyFamily::Typography),
			("font-variant", PropertyFamily::Typography),
			("font-weight", PropertyFamily::Typography),
			("line-height", PropertyFamily::Typography),
			("letter-spacing", PropertyFamily::Typography),
			("font", PropertyFamily::Typography),
			("text-align", PropertyFamily::Typography),
			("text-overflow", PropertyFamily::Typography),
			("text-decoration", PropertyFamily::Typography),
			("text-transform", PropertyFamily::Typography),
			("text-wrap", PropertyFamily::Typography),
			("white-space", PropertyFamily::Typography),
			("word-break", PropertyFamily::Typography),
			("background-color", PropertyFamily::BackgroundAndBorder),
			("background-image", PropertyFamily::BackgroundAndBorder),
			("background-position", PropertyFamily::BackgroundAndBorder),
			("background-repeat", PropertyFamily::BackgroundAndBorder),
			("background-size", PropertyFamily::BackgroundAndBorder),
			("background", PropertyFamily::BackgroundAndBorder),
			("border", PropertyFamily::BackgroundAndBorder),
			("border-top", PropertyFamily::BackgroundAndBorder),
			("border-right", PropertyFamily::BackgroundAndBorder),
			("border-bottom", PropertyFamily::BackgroundAndBorder),
			("border-left", PropertyFamily::BackgroundAndBorder),
			("border-width", PropertyFamily::BackgroundAndBorder),
			("border-top-width", PropertyFamily::BackgroundAndBorder),
			("border-right-width", PropertyFamily::BackgroundAndBorder),
			("border-bottom-width", PropertyFamily::BackgroundAndBorder),
			("border-left-width", PropertyFamily::BackgroundAndBorder),
			("border-style", PropertyFamily::BackgroundAndBorder),
			("border-top-style", PropertyFamily::BackgroundAndBorder),
			("border-right-style", PropertyFamily::BackgroundAndBorder),
			("border-bottom-style", PropertyFamily::BackgroundAndBorder),
			("border-left-style", PropertyFamily::BackgroundAndBorder),
			("border-color", PropertyFamily::BackgroundAndBorder),
			("border-top-color", PropertyFamily::BackgroundAndBorder),
			("border-right-color", PropertyFamily::BackgroundAndBorder),
			("border-bottom-color", PropertyFamily::BackgroundAndBorder),
			("border-left-color", PropertyFamily::BackgroundAndBorder),
			("border-radius", PropertyFamily::BackgroundAndBorder),
			(
				"border-top-left-radius",
				PropertyFamily::BackgroundAndBorder,
			),
			(
				"border-top-right-radius",
				PropertyFamily::BackgroundAndBorder,
			),
			(
				"border-bottom-right-radius",
				PropertyFamily::BackgroundAndBorder,
			),
			(
				"border-bottom-left-radius",
				PropertyFamily::BackgroundAndBorder,
			),
			("box-shadow", PropertyFamily::Effects),
			("opacity", PropertyFamily::Effects),
			("outline", PropertyFamily::Effects),
			("outline-color", PropertyFamily::Effects),
			("outline-offset", PropertyFamily::Effects),
			("outline-style", PropertyFamily::Effects),
			("outline-width", PropertyFamily::Effects),
			("filter", PropertyFamily::Effects),
			("backdrop-filter", PropertyFamily::Effects),
			("transform", PropertyFamily::TransformAndTransition),
			("transform-origin", PropertyFamily::TransformAndTransition),
			(
				"transition-property",
				PropertyFamily::TransformAndTransition,
			),
			(
				"transition-duration",
				PropertyFamily::TransformAndTransition,
			),
			(
				"transition-timing-function",
				PropertyFamily::TransformAndTransition,
			),
			("transition-delay", PropertyFamily::TransformAndTransition),
			("transition", PropertyFamily::TransformAndTransition),
			("cursor", PropertyFamily::InteractionAndGenerated),
			("pointer-events", PropertyFamily::InteractionAndGenerated),
			("resize", PropertyFamily::InteractionAndGenerated),
			("touch-action", PropertyFamily::InteractionAndGenerated),
			("user-select", PropertyFamily::InteractionAndGenerated),
			("content", PropertyFamily::InteractionAndGenerated),
			(
				"list-style-position",
				PropertyFamily::InteractionAndGenerated,
			),
			("list-style-type", PropertyFamily::InteractionAndGenerated),
			("list-style", PropertyFamily::InteractionAndGenerated),
		];

		// Act
		let actual = property_specs()
			.iter()
			.map(|spec| (spec.name, spec.family))
			.collect::<Vec<_>>();

		// Assert
		assert_eq!(actual, expected);
	}

	#[rstest]
	fn property_registry_names_are_unique_canonical_and_unprefixed() {
		// Arrange
		let specs = property_specs();

		// Act
		let names = specs.iter().map(|spec| spec.name).collect::<Vec<_>>();
		let unique = names.iter().copied().collect::<BTreeSet<_>>();
		let canonical = names.iter().all(|name| {
			!name.starts_with('-')
				&& !name.ends_with('-')
				&& name
					.bytes()
					.all(|byte| byte.is_ascii_lowercase() || byte == b'-')
				&& !name.contains("--")
		});

		// Assert
		assert_eq!(unique.len(), names.len());
		assert!(canonical);
	}

	#[rstest]
	fn every_property_has_exact_css_wide_keyword_domain() {
		// Arrange
		let expected = ["inherit", "initial", "unset", "revert", "revert-layer"];

		// Act
		let actual = property_specs()
			.iter()
			.map(|spec| spec.css_wide_keywords.keywords)
			.collect::<Vec<_>>();

		// Assert
		assert_eq!(actual, vec![expected.as_slice(); property_specs().len()]);
	}

	#[rstest]
	fn every_property_keyword_domain_produces_keyword_semantics() {
		// Arrange
		let specs = property_specs();

		// Act
		let actual = specs
			.iter()
			.flat_map(|spec| {
				let mut domains = vec![spec.css_wide_keywords];
				collect_keyword_domains(spec.grammar, &mut domains);
				domains
			})
			.map(|domain| (domain.name, domain.produced_type))
			.collect::<Vec<_>>();

		// Assert
		assert!(
			actual
				.iter()
				.all(|(_, produced_type)| *produced_type == SemanticType::Keyword)
		);
	}

	#[rstest]
	#[case("--custom")]
	#[case("-webkit-transform")]
	#[case("unknown")]
	fn property_lookup_rejects_non_mvp_names(#[case] name: &str) {
		// Arrange and Act
		let actual = property_spec(name);

		// Assert
		assert_eq!(actual, None);
	}

	#[rstest]
	#[case(
		"border",
		"UNORDERED(min=1,source-order=false,width?:OR(NON_NEGATIVE(LENGTH),KW(line-width)),style?:KW(line-style),color?:COLOR)"
	)]
	#[case(
		"flex",
		"OR(KW(flex),OR(NON_NEGATIVE(LENGTH_PERCENTAGE),KW(size)),ORDERED(grow:NON_NEGATIVE(NUMBER),shrink?:NON_NEGATIVE(NUMBER),basis?:OR(NON_NEGATIVE(LENGTH_PERCENTAGE),KW(size))))"
	)]
	#[case(
		"flex-flow",
		"UNORDERED(min=1,source-order=false,direction?:KW(flex-direction),wrap?:KW(flex-wrap))"
	)]
	#[case(
		"grid-auto-flow",
		"UNORDERED(min=1,source-order=false,axis?:KW(grid-flow-axis),density?:KW(dense))"
	)]
	#[case(
		"grid",
		"OR(KW(none),SLASH(SPACE(1,*,OR(NON_NEGATIVE(LENGTH_PERCENTAGE),NON_NEGATIVE(GRID_FRACTION),KW(track))),SPACE(1,*,OR(NON_NEGATIVE(LENGTH_PERCENTAGE),NON_NEGATIVE(GRID_FRACTION),KW(track)))))"
	)]
	#[case(
		"border-radius",
		"OR(SPACE(1,4,NON_NEGATIVE(LENGTH_PERCENTAGE)),SLASH(SPACE(1,4,NON_NEGATIVE(LENGTH_PERCENTAGE)),SPACE(1,4,NON_NEGATIVE(LENGTH_PERCENTAGE))))"
	)]
	#[case(
		"transition",
		"COMMA(1,UNORDERED(min=1,source-order=true,property?:OR(KW(transition-property),IDENT),duration?:NON_NEGATIVE(TIME),timing-function?:KW(timing),delay?:TIME))"
	)]
	fn key_shorthand_has_exact_structural_grammar(#[case] property: &str, #[case] expected: &str) {
		// Arrange
		let spec = property_spec(property).expect("registered property");

		// Act
		let actual = spec.grammar.describe();

		// Assert
		assert_eq!(actual, expected);
	}

	#[rstest]
	fn function_registry_has_exact_complete_specs() {
		// Arrange
		let expected = [
			"min|css=min|arity=at-least:2|args=repeated:NUMERIC(JOINED)|receiver=none|result=JOINED_NUMERIC|lowering=comma-function",
			"max|css=max|arity=at-least:2|args=repeated:NUMERIC(JOINED)|receiver=none|result=JOINED_NUMERIC|lowering=comma-function",
			"clamp|css=clamp|arity=exact:3|args=positional:[NUMERIC(JOINED),NUMERIC(JOINED),NUMERIC(JOINED)]|receiver=none|result=JOINED_NUMERIC|lowering=comma-function",
			"Color::rgb|css=rgb|arity=exact:3|args=positional:[NUMERIC(NUMBER_OR_PERCENTAGE),NUMERIC(NUMBER_OR_PERCENTAGE),NUMERIC(NUMBER_OR_PERCENTAGE)]|receiver=none|result=COLOR|lowering=space-function",
			"Color::hsl|css=hsl|arity=exact:3|args=positional:[ANGLE,PERCENTAGE,PERCENTAGE]|receiver=none|result=COLOR|lowering=space-function",
			"Color::oklch|css=oklch|arity=exact:3|args=positional:[NUMERIC(NUMBER_OR_PERCENTAGE),NUMERIC(NUMBER_OR_PERCENTAGE),ANGLE]|receiver=none|result=COLOR|lowering=space-function",
			".mix|css=color-mix|arity=exact:2|args=positional:[COLOR,NUMERIC(PERCENTAGE_RANGE(0,100))]|receiver=COLOR|result=COLOR|lowering=color-mix-srgb",
			"stop|css=<color> <position>|arity=exact:2|args=positional:[COLOR,LENGTH_PERCENTAGE]|receiver=none|result=GRADIENT_STOP|lowering=space-pair",
			"linear_gradient|css=linear-gradient|arity=exact:2|args=positional:[DIRECTION,COMMA_LIST(min=2,GRADIENT_STOP)]|receiver=none|result=IMAGE|lowering=comma-function",
			"translate|css=translate|arity=exact:2|args=positional:[LENGTH_PERCENTAGE,LENGTH_PERCENTAGE]|receiver=none|result=TRANSFORM_FUNCTION|lowering=comma-function",
			"translate_x|css=translateX|arity=exact:1|args=positional:[LENGTH_PERCENTAGE]|receiver=none|result=TRANSFORM_FUNCTION|lowering=comma-function",
			"translate_y|css=translateY|arity=exact:1|args=positional:[LENGTH_PERCENTAGE]|receiver=none|result=TRANSFORM_FUNCTION|lowering=comma-function",
			"rotate|css=rotate|arity=exact:1|args=positional:[ANGLE]|receiver=none|result=TRANSFORM_FUNCTION|lowering=comma-function",
			"scale|css=scale|arity=exact:1|args=positional:[NUMBER]|receiver=none|result=TRANSFORM_FUNCTION|lowering=comma-function",
			"scale_x|css=scaleX|arity=exact:1|args=positional:[NUMBER]|receiver=none|result=TRANSFORM_FUNCTION|lowering=comma-function",
			"scale_y|css=scaleY|arity=exact:1|args=positional:[NUMBER]|receiver=none|result=TRANSFORM_FUNCTION|lowering=comma-function",
			"slash|css=<left> / <right>|arity=exact:2|args=positional:[ANY,ANY]|receiver=none|result=SLASH_PAIR|lowering=slash-pair",
		];

		// Act
		let actual = function_specs()
			.iter()
			.map(|spec| spec.describe())
			.collect::<Vec<_>>();
		let unique = function_specs()
			.iter()
			.map(|spec| spec.dsl_path)
			.collect::<BTreeSet<_>>();

		// Assert
		assert_eq!(actual, expected);
		assert_eq!(unique.len(), expected.len());
	}

	#[rstest]
	#[case("var", ReservedFunction::Var, StyleDiagnosticKind::DirectVarCall)]
	#[case("calc", ReservedFunction::Calc, StyleDiagnosticKind::DirectCalcCall)]
	fn direct_css_functions_have_reserved_diagnostics(
		#[case] name: &str,
		#[case] expected_reserved: ReservedFunction,
		#[case] expected_diagnostic: StyleDiagnosticKind,
	) {
		// Arrange and Act
		let spec = function_spec(name);
		let reserved = reserved_function(name);

		// Assert
		assert_eq!(spec, None);
		assert_eq!(reserved, Some(expected_reserved));
		assert_eq!(
			reserved.map(ReservedFunction::diagnostic_kind),
			Some(expected_diagnostic)
		);
	}

	#[rstest]
	#[case("Color::rgb", None, &["20%", "30", "40%"], "rgb(20% 30 40%)")]
	#[case("Color::hsl", None, &["120deg", "50%", "25%"], "hsl(120deg 50% 25%)")]
	#[case("Color::oklch", None, &["60%", "0.2", "40deg"], "oklch(60% 0.2 40deg)")]
	#[case("translate", None, &["1rem", "25%"], "translate(1rem, 25%)")]
	#[case("stop", None, &["red", "25%"], "red 25%")]
	#[case("slash", None, &["1fr", "2fr"], "1fr / 2fr")]
	#[case(
		"linear_gradient",
		None,
		&["to right", "red 0%, blue 100%"],
		"linear-gradient(to right, red 0%, blue 100%)"
	)]
	#[case(
		".mix",
		Some("red"),
		&["blue", "30%"],
		"color-mix(in srgb, red calc(100% - 30%), blue 30%)"
	)]
	fn function_lowering_uses_rendered_fragments(
		#[case] dsl_path: &str,
		#[case] receiver: Option<&str>,
		#[case] arguments: &[&str],
		#[case] expected: &str,
	) {
		// Arrange
		let spec = function_spec(dsl_path).expect("registered function");

		// Act
		let actual = spec.lower_rendered(receiver, arguments);

		// Assert
		assert_eq!(actual.as_deref(), Some(expected));
	}

	#[rstest]
	fn color_mix_lowering_uses_registry_css_spelling() {
		// Arrange
		let mut spec = *function_spec(".mix").expect("registered function");
		spec.css_spelling = "custom-color-mix";

		// Act
		let actual = spec.lower_rendered(Some("red"), &["blue", "30%"]);

		// Assert
		assert_eq!(
			actual.as_deref(),
			Some("custom-color-mix(in srgb, red calc(100% - 30%), blue 30%)")
		);
	}

	#[rstest]
	#[case("px", Some(("px", NumericDimension::Length)))]
	#[case("fr", Some(("fr", NumericDimension::GridFraction)))]
	#[case("number", None)]
	#[case("integer", None)]
	#[case("", None)]
	fn unit_lookup_is_exact_and_keeps_scalars_unitless(
		#[case] name: &str,
		#[case] expected: Option<(&str, NumericDimension)>,
	) {
		// Arrange and Act
		let actual = unit_spec(name).map(|spec| (spec.name, spec.dimension));

		// Assert
		assert_eq!(actual, expected);
	}

	#[rstest]
	fn registry_reference_is_deterministic_and_lists_table_names() {
		// Arrange
		let expected_properties = property_specs()
			.iter()
			.map(|spec| spec.name)
			.collect::<Vec<_>>();
		let expected_units = unit_specs()
			.iter()
			.map(|spec| spec.name)
			.collect::<Vec<_>>();
		let expected_functions = function_specs()
			.iter()
			.map(|spec| spec.dsl_path)
			.collect::<Vec<_>>();

		// Act
		let first = registry_reference_text();
		let second = registry_reference_text();
		let listed_properties = reference_names(&first, "property\t");
		let listed_units = reference_names(&first, "unit\t");
		let listed_functions = reference_names(&first, "function\t");
		let listed_named_colors = reference_names(&first, "named-color\t");

		// Assert
		assert_eq!(first, second);
		assert_eq!(listed_properties, expected_properties);
		assert_eq!(listed_units, expected_units);
		assert_eq!(listed_functions, expected_functions);
		assert_eq!(listed_named_colors, named_color_domain().keywords);
	}

	#[rstest]
	fn registry_reference_has_exact_representative_payload_lines() {
		// Arrange
		let reference = registry_reference_text();
		let lines = reference.lines().collect::<BTreeSet<_>>();

		// Act and Assert
		assert!(lines.contains("[named-colors]"));
		assert!(lines.contains("named-color\tred\tCOLOR"));
		assert!(lines.contains(
			"property\tdisplay\tLayout\tKW(display:[none|contents|block|inline|inline-block|flow-root|flex|inline-flex|grid|inline-grid|table|table-row|table-cell|list-item])\tcss-wide=[inherit|initial|unset|revert|revert-layer]"
		));
		assert!(lines.contains(
			"function\tColor::rgb\tColor::rgb|css=rgb|arity=exact:3|args=positional:[NUMERIC(NUMBER_OR_PERCENTAGE),NUMERIC(NUMBER_OR_PERCENTAGE),NUMERIC(NUMBER_OR_PERCENTAGE)]|receiver=none|result=COLOR|lowering=space-function"
		));
	}

	fn reference_names<'a>(reference: &'a str, prefix: &str) -> Vec<&'a str> {
		reference
			.lines()
			.filter_map(|line| line.strip_prefix(prefix))
			.map(|line| line.split_once('\t').map_or(line, |(name, _)| name))
			.collect()
	}

	fn collect_keyword_domains<'a>(
		grammar: &'a ValueGrammar,
		domains: &mut Vec<&'a crate::core::KeywordDomain>,
	) {
		match grammar {
			ValueGrammar::Keyword(domain) => domains.push(domain),
			ValueGrammar::NonNegative(grammar) | ValueGrammar::NumericRange { grammar, .. } => {
				collect_keyword_domains(grammar, domains)
			}
			ValueGrammar::Or(alternatives) => {
				for alternative in *alternatives {
					collect_keyword_domains(alternative, domains);
				}
			}
			ValueGrammar::Space { item, .. }
			| ValueGrammar::Comma { item, .. }
			| ValueGrammar::SlashList { item, .. } => collect_keyword_domains(item, domains),
			ValueGrammar::CommaFinal {
				item, final_item, ..
			} => {
				collect_keyword_domains(item, domains);
				collect_keyword_domains(final_item, domains);
			}
			ValueGrammar::Slash { left, right } => {
				collect_keyword_domains(left, domains);
				collect_keyword_domains(right, domains);
			}
			ValueGrammar::Ordered(members) | ValueGrammar::Unordered { members, .. } => {
				for member in *members {
					collect_keyword_domains(member.grammar, domains);
				}
			}
			ValueGrammar::Primitive(_)
			| ValueGrammar::Identifier
			| ValueGrammar::IdentifierExcept(_)
			| ValueGrammar::FunctionResult(_) => {}
		}
	}
}
