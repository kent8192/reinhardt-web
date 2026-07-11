//! Untyped AST nodes for the `style!` macro DSL.

use proc_macro2::{Delimiter, Literal, Span, TokenTree};

/// The complete untyped contents of one `style!` invocation.
#[derive(Debug, Clone)]
pub struct StyleMacro {
	/// Global custom-property declarations in source order.
	pub globals: Vec<StyleGlobalDeclaration>,
	/// Component variable declarations in source order.
	pub variables: Vec<StyleVariableDeclaration>,
	/// Top-level rules and media rules in source order.
	pub items: Vec<StyleItem>,
	/// Span used to report errors for the complete definition.
	pub span: Span,
}

/// A source identifier used by the style variable namespaces.
#[derive(Debug, Clone)]
pub struct StyleBindingName {
	/// The identifier exactly as written in the style definition.
	pub value: String,
	/// Span of the source identifier.
	pub span: Span,
}

impl StyleBindingName {
	/// Returns the source spelling of this name.
	pub fn as_str(&self) -> &str {
		&self.value
	}
}

/// A syntactic DSL type name awaiting semantic resolution.
#[derive(Debug, Clone)]
pub struct StyleDslType {
	/// The type name exactly as written in the style definition.
	pub name: String,
	/// Span of the source type name.
	pub span: Span,
}

impl StyleDslType {
	/// Returns the source spelling of this type name.
	pub fn as_str(&self) -> &str {
		&self.name
	}
}

/// A canonical CSS name assembled from kebab-case source tokens.
#[derive(Debug, Clone)]
pub struct CssName {
	/// The canonical kebab-case name.
	pub value: String,
	/// Span covering the source name when available.
	pub span: Span,
}

impl CssName {
	/// Returns the canonical CSS spelling.
	pub fn as_str(&self) -> &str {
		&self.value
	}
}

/// One declaration in the `globals` block.
#[derive(Debug, Clone)]
pub struct StyleGlobalDeclaration {
	/// Name used to reference the global from style values.
	pub name: StyleBindingName,
	/// Declared DSL type.
	pub ty: StyleDslType,
	/// Span of the declaration.
	pub span: Span,
}

/// One declaration in the `vars` block.
#[derive(Debug, Clone)]
pub struct StyleVariableDeclaration {
	/// Name used to reference the variable from style values.
	pub name: StyleBindingName,
	/// Declared DSL type.
	pub ty: StyleDslType,
	/// Authored default expression, retained as absent for semantic diagnostics.
	pub default: Option<StyleValueExpression>,
	/// Span of the declaration.
	pub span: Span,
}

/// A top-level source item in a style definition.
#[derive(Debug, Clone)]
pub enum StyleItem {
	/// A normal style rule.
	Rule(StyleRule),
	/// A media rule.
	Media(StyleMediaRule),
}

impl StyleItem {
	/// Returns the source span of this item.
	pub fn span(&self) -> Span {
		match self {
			Self::Rule(rule) => rule.span,
			Self::Media(rule) => rule.span,
		}
	}
}

/// One item inside a rule or media-rule body.
#[derive(Debug, Clone)]
pub enum StyleRuleItem {
	/// A property declaration.
	Declaration(StyleDeclaration),
	/// A structurally nested rule.
	Rule(StyleRule),
	/// A structurally nested media rule.
	Media(StyleMediaRule),
}

impl StyleRuleItem {
	/// Returns the source span of this item.
	pub fn span(&self) -> Span {
		match self {
			Self::Declaration(declaration) => declaration.span,
			Self::Rule(rule) => rule.span,
			Self::Media(rule) => rule.span,
		}
	}
}

/// An untyped CSS property declaration.
#[derive(Debug, Clone)]
pub struct StyleDeclaration {
	/// Canonical CSS property name.
	pub name: CssName,
	/// Structurally parsed value expression.
	pub value: StyleValueExpression,
	/// Span of the declaration.
	pub span: Span,
}

/// A structurally parsed style rule.
#[derive(Debug, Clone)]
pub struct StyleRule {
	/// Selector list preceding the rule body.
	pub selectors: StyleSelectorList,
	/// Declarations and nested structural rules in source order.
	pub items: Vec<StyleRuleItem>,
	/// Span of the rule head.
	pub span: Span,
}

/// A comma-separated selector list awaiting semantic selector parsing.
#[derive(Debug, Clone)]
pub struct StyleSelectorList {
	/// Selectors in source order.
	pub selectors: Vec<StyleSelector>,
	/// Span of the selector list.
	pub span: Span,
}

/// One structurally parsed selector branch.
#[derive(Debug, Clone)]
pub struct StyleSelector {
	/// Structural role and simple-selector contents.
	pub kind: StyleSelectorKind,
	/// Span of the selector expression.
	pub span: Span,
}

/// The structural role of one selector branch.
#[derive(Debug, Clone)]
pub enum StyleSelectorKind {
	/// A selector at the root of a top-level rule.
	Root(StyleSimpleSelector),
	/// A nested selector that refines the same element through `&`.
	SameElement(StyleSimpleSelector),
	/// A nested selector related to its parent through a combinator.
	Relative {
		/// Relationship to the parent selector.
		combinator: StyleSelectorCombinator,
		/// The single simple selector at the relationship target.
		selector: StyleSimpleSelector,
	},
}

/// A selector relationship that creates a distinct target element.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleSelectorCombinator {
	/// An implicitly nested descendant.
	Descendant,
	/// A direct child introduced by `>`.
	Child,
	/// An adjacent sibling introduced by `+`.
	AdjacentSibling,
	/// A general sibling introduced by `~`.
	GeneralSibling,
}

/// One simple selector accepted by a structural selector head.
#[derive(Debug, Clone)]
pub enum StyleSimpleSelector {
	/// A local class selector.
	Class(StyleSelectorName),
	/// A type selector.
	Type(StyleSelectorName),
	/// An ID selector.
	Id(StyleSelectorName),
	/// The universal selector.
	Universal {
		/// Span of the `*` token.
		span: Span,
	},
	/// An attribute selector.
	Attribute(StyleAttributeSelector),
	/// A pseudo-class or pseudo-function selector.
	Pseudo(StylePseudoSelector),
}

impl StyleSimpleSelector {
	/// Returns the complete source span of this selector.
	pub fn span(&self) -> Span {
		match self {
			Self::Class(name) | Self::Type(name) | Self::Id(name) => name.span,
			Self::Universal { span } => *span,
			Self::Attribute(attribute) => attribute.span,
			Self::Pseudo(pseudo) => pseudo.span,
		}
	}
}

/// A name used inside a selector.
#[derive(Debug, Clone)]
pub struct StyleSelectorName {
	/// Source spelling assembled from identifier and hyphen tokens.
	pub value: String,
	/// Span covering the complete source name.
	pub span: Span,
}

impl StyleSelectorName {
	/// Returns the source spelling of this selector name.
	pub fn as_str(&self) -> &str {
		&self.value
	}
}

/// One structurally parsed attribute selector.
#[derive(Debug, Clone)]
pub struct StyleAttributeSelector {
	/// Attribute name.
	pub name: StyleSelectorName,
	/// Optional matcher operator.
	pub matcher: Option<StyleAttributeMatcher>,
	/// Optional matcher value.
	pub value: Option<StyleAttributeValue>,
	/// Optional ASCII case-sensitivity modifier.
	pub modifier: Option<StyleSelectorName>,
	/// Span of the complete bracketed selector.
	pub span: Span,
}

/// A CSS attribute-selector matcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleAttributeMatcher {
	/// Exact equality (`=`).
	Equals,
	/// Whitespace-separated token membership (`~=`).
	Includes,
	/// Exact value or hyphen-prefixed value (`|=`).
	DashMatch,
	/// Prefix match (`^=`).
	Prefix,
	/// Suffix match (`$=`).
	Suffix,
	/// Substring match (`*=`).
	Substring,
}

/// A static attribute-selector value.
#[derive(Debug, Clone)]
pub enum StyleAttributeValue {
	/// An unquoted identifier value.
	Identifier(StyleSelectorName),
	/// A quoted string value.
	String {
		/// Decoded string contents.
		value: String,
		/// Span of the string literal.
		span: Span,
	},
}

/// One pseudo-class or pseudo-function selector.
#[derive(Debug, Clone)]
pub struct StylePseudoSelector {
	/// Pseudo selector name without the leading colon.
	pub name: StyleSelectorName,
	/// Optional parenthesized token tree for a pseudo-function.
	pub arguments: Option<StyleSelectorArguments>,
	/// Span of the complete pseudo selector.
	pub span: Span,
}

/// The token structure inside one selector pseudo-function.
#[derive(Debug, Clone)]
pub struct StyleSelectorArguments {
	/// Argument tokens in source order, including nested token groups.
	pub tokens: Vec<TokenTree>,
	/// Structured selector branches for known selector-list pseudo-functions.
	pub selector_list: Option<StyleSelectorList>,
	/// Structured formula metadata for `nth-child` and `nth-last-child`.
	pub nth: Option<StyleNthSelectorArguments>,
	/// Span of the parenthesized argument group.
	pub span: Span,
}

/// The formula prefix and optional selector separator of an nth pseudo-function.
#[derive(Debug, Clone)]
pub struct StyleNthSelectorArguments {
	/// Losslessly retained An+B or keyword formula tokens before `of`.
	pub formula_tokens: Vec<TokenTree>,
	/// Span covering the formula prefix.
	pub formula_span: Span,
	/// Span of the optional case-insensitive `of` separator.
	pub of_span: Option<Span>,
}

/// A structurally parsed `@media` rule.
#[derive(Debug, Clone)]
pub struct StyleMediaRule {
	/// Untyped media condition.
	pub condition: StyleMediaCondition,
	/// Body items in source order.
	pub items: Vec<StyleRuleItem>,
	/// Span of the `@media` rule.
	pub span: Span,
}

/// A statically parsed media condition.
#[derive(Debug, Clone)]
pub struct StyleMediaCondition {
	/// Static media tokens in source order.
	pub tokens: Vec<StyleMediaToken>,
	/// Span of the condition.
	pub span: Span,
}

/// One token in the static media-condition tree.
#[derive(Debug, Clone)]
pub enum StyleMediaToken {
	/// A media type, feature, keyword, or other CSS identifier.
	Identifier(StyleMediaIdentifier),
	/// A boolean media operator.
	Operator(StyleMediaOperator),
	/// A numeric literal with an optional CSS unit suffix.
	Number(StyleMediaNumber),
	/// Static CSS punctuation.
	Punctuation(StyleMediaPunctuation),
	/// A nested parenthesized media expression.
	Parenthesized(StyleMediaGroup),
}

impl StyleMediaToken {
	/// Returns the source span of this token.
	pub fn span(&self) -> Span {
		match self {
			Self::Identifier(identifier) => identifier.span,
			Self::Operator(operator) => operator.span,
			Self::Number(number) => number.span,
			Self::Punctuation(punctuation) => punctuation.span,
			Self::Parenthesized(group) => group.span,
		}
	}
}

/// An identifier in a media condition.
#[derive(Debug, Clone)]
pub struct StyleMediaIdentifier {
	/// Source spelling, including kebab separators.
	pub value: String,
	/// Span covering the complete identifier.
	pub span: Span,
}

impl StyleMediaIdentifier {
	/// Returns the source spelling of this identifier.
	pub fn as_str(&self) -> &str {
		&self.value
	}
}

/// A boolean operator in a media condition.
#[derive(Debug, Clone)]
pub struct StyleMediaOperator {
	/// Operator kind.
	pub kind: StyleMediaOperatorKind,
	/// Span of the source operator.
	pub span: Span,
}

/// Boolean media operators supported by the static grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleMediaOperatorKind {
	/// Logical conjunction.
	And,
	/// Logical disjunction.
	Or,
	/// Logical negation.
	Not,
	/// Restricts a media type to supporting user agents.
	Only,
}

/// A numeric media value.
#[derive(Debug, Clone)]
pub struct StyleMediaNumber {
	/// Original numeric literal token.
	pub literal: Literal,
	/// Numeric digits without the unit suffix.
	pub value: String,
	/// Optional CSS unit suffix.
	pub unit: Option<String>,
	/// Numeric token kind.
	pub kind: StyleMediaNumberKind,
	/// Span of the literal.
	pub span: Span,
}

/// The lexical form of a media number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleMediaNumberKind {
	/// An integer literal.
	Integer,
	/// A floating-point literal.
	Float,
}

/// One punctuation token in a media condition.
#[derive(Debug, Clone)]
pub struct StyleMediaPunctuation {
	/// Punctuation kind.
	pub kind: StyleMediaPunctuationKind,
	/// Span of the source punctuation.
	pub span: Span,
}

/// Static CSS punctuation accepted in media conditions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleMediaPunctuationKind {
	/// Feature/value separator (`:`).
	Colon,
	/// Aspect-ratio separator (`/`).
	Slash,
	/// Media-list separator (`,`).
	Comma,
	/// Percentage suffix (`%`).
	Percent,
	/// Less-than comparison (`<`).
	LessThan,
	/// Less-than-or-equal comparison (`<=`).
	LessThanOrEqual,
	/// Greater-than comparison (`>`).
	GreaterThan,
	/// Greater-than-or-equal comparison (`>=`).
	GreaterThanOrEqual,
	/// Equality comparison (`=`).
	Equal,
	/// A positive numeric sign (`+`).
	Plus,
	/// A negative numeric sign (`-`).
	Minus,
}

/// One parenthesized subtree in a media condition.
#[derive(Debug, Clone)]
pub struct StyleMediaGroup {
	/// Static tokens inside the parentheses in source order.
	pub tokens: Vec<StyleMediaToken>,
	/// Span of the parenthesized group.
	pub span: Span,
}

/// One parsed value expression awaiting semantic validation.
#[derive(Debug, Clone)]
pub struct StyleValueExpression {
	/// Structural expression kind.
	pub kind: StyleValueExpr,
	/// Span of the value expression.
	pub span: Span,
}

/// The structural forms accepted by the `style!` value language.
#[derive(Debug, Clone)]
pub enum StyleValueExpr {
	/// A typed literal.
	Literal(StyleValueLiteral),
	/// A reference through the `globals` or `vars` namespace.
	QualifiedReference(StyleQualifiedReference),
	/// An associated path used as a value, such as `Direction::Right`.
	AssociatedPathValue(StyleValuePath),
	/// A signed expression.
	Unary(StyleUnaryExpression),
	/// An arithmetic expression.
	Binary(StyleBinaryExpression),
	/// A free function or associated constructor call.
	Call(StyleValueCall),
	/// A receiver method call.
	MethodCall(StyleMethodCall),
	/// One explicitly grouped expression.
	Group(StyleGroupedValue),
	/// A comma-authored sequence that lowers to CSS spaces.
	SpaceSequence(StyleValueCollection),
	/// A bracket-authored list that lowers to CSS commas.
	CommaList(StyleValueCollection),
	/// The explicitly unchecked whole-value function escape.
	UncheckedFunction(StyleUncheckedFunction),
}

/// A source identifier in a value expression.
#[derive(Debug, Clone)]
pub struct StyleValueName {
	/// Identifier spelling exactly as authored.
	pub value: String,
	/// Span of the identifier.
	pub span: Span,
}

impl StyleValueName {
	/// Returns the source spelling of this name.
	pub fn as_str(&self) -> &str {
		&self.value
	}
}

/// A literal accepted by the value-expression parser.
#[derive(Debug, Clone)]
pub enum StyleValueLiteral {
	/// An integer literal with an optional CSS unit.
	Integer(StyleNumericLiteral),
	/// A floating-point literal with an optional CSS unit.
	Number(StyleNumericLiteral),
	/// A three-, four-, six-, or eight-digit hexadecimal color.
	HexColor(StyleHexColorLiteral),
	/// A CSS keyword or custom identifier awaiting registry validation.
	Keyword(StyleValueName),
	/// A quoted CSS string.
	String(StyleStringLiteral),
}

/// One integer or floating-point literal.
#[derive(Debug, Clone)]
pub struct StyleNumericLiteral {
	/// Numeric lexeme without its suffix unit.
	pub source: String,
	/// Optional CSS suffix unit.
	pub unit: Option<StyleNumericUnit>,
	/// Whether this is the unitless contextual-zero form.
	pub contextual_zero: bool,
	/// Span of the authored numeric token and percentage marker when present.
	pub span: Span,
}

/// A syntactically attached CSS numeric unit.
#[derive(Debug, Clone)]
pub enum StyleNumericUnit {
	/// A suffix carried by the Rust integer or float token.
	Named(StyleValueName),
	/// A `%` punctuation suffix.
	Percentage {
		/// Span of the `%` punctuation.
		span: Span,
	},
}

/// A hexadecimal color literal that retains its exact spelling.
#[derive(Debug, Clone)]
pub struct StyleHexColorLiteral {
	/// Complete source spelling, including `#` and authored letter case.
	pub source: String,
	/// Hexadecimal digits without the leading `#`.
	pub digits: String,
	/// Span covering the complete literal.
	pub span: Span,
}

/// One quoted CSS string literal.
#[derive(Debug, Clone)]
pub struct StyleStringLiteral {
	/// Exact Rust string-literal spelling.
	pub source: String,
	/// Decoded string contents.
	pub value: String,
	/// Span of the string literal.
	pub span: Span,
}

/// The namespace of a qualified style binding reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleReferenceNamespace {
	/// The package-provided global custom-property namespace.
	Globals,
	/// The component variable namespace.
	Variables,
}

/// A reference such as `globals.border` or `vars.gutter`.
#[derive(Debug, Clone)]
pub struct StyleQualifiedReference {
	/// Structured namespace kind.
	pub namespace: StyleReferenceNamespace,
	/// Span of the namespace identifier.
	pub namespace_span: Span,
	/// Span of the `.` separator.
	pub dot_span: Span,
	/// Referenced binding name.
	pub name: StyleValueName,
	/// Span covering the complete reference.
	pub span: Span,
}

/// An identifier path separated by Rust `::` punctuation.
#[derive(Debug, Clone)]
pub struct StyleValuePath {
	/// Path segments in source order.
	pub segments: Vec<StyleValueName>,
	/// Spans of the `::` separators in source order.
	pub separator_spans: Vec<Span>,
	/// Span covering the complete path.
	pub span: Span,
}

/// One unary operator application.
#[derive(Debug, Clone)]
pub struct StyleUnaryExpression {
	/// Authored unary operator.
	pub operator: StyleUnaryOperator,
	/// Operand to which the sign binds.
	pub operand: Box<StyleValueExpression>,
	/// Span covering the complete expression.
	pub span: Span,
}

/// A unary operator and its source span.
#[derive(Debug, Clone)]
pub struct StyleUnaryOperator {
	/// Operator kind.
	pub kind: StyleUnaryOperatorKind,
	/// Span of the operator token.
	pub span: Span,
}

/// Unary arithmetic operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleUnaryOperatorKind {
	/// Unary positive sign.
	Plus,
	/// Unary negative sign.
	Minus,
}

/// One binary arithmetic operation.
#[derive(Debug, Clone)]
pub struct StyleBinaryExpression {
	/// Left operand.
	pub left: Box<StyleValueExpression>,
	/// Authored binary operator.
	pub operator: StyleBinaryOperator,
	/// Right operand.
	pub right: Box<StyleValueExpression>,
	/// Span covering the complete expression.
	pub span: Span,
}

/// A binary operator and its source span.
#[derive(Debug, Clone)]
pub struct StyleBinaryOperator {
	/// Operator kind.
	pub kind: StyleBinaryOperatorKind,
	/// Span of the operator token.
	pub span: Span,
}

/// Binary arithmetic operators ordered by the parser's binding powers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleBinaryOperatorKind {
	/// Addition.
	Add,
	/// Subtraction.
	Subtract,
	/// Multiplication.
	Multiply,
	/// Division.
	Divide,
}

/// A free function or associated constructor call.
#[derive(Debug, Clone)]
pub struct StyleValueCall {
	/// Structured function or constructor path.
	pub path: StyleValuePath,
	/// Ordered call arguments.
	pub arguments: Vec<StyleValueExpression>,
	/// Span of the parenthesized arguments.
	pub arguments_span: Span,
	/// Span covering the complete call.
	pub span: Span,
}

/// A method call that retains its receiver expression.
#[derive(Debug, Clone)]
pub struct StyleMethodCall {
	/// Receiver expression.
	pub receiver: Box<StyleValueExpression>,
	/// Method name.
	pub method: StyleValueName,
	/// Ordered call arguments.
	pub arguments: Vec<StyleValueExpression>,
	/// Span of the `.` punctuation.
	pub dot_span: Span,
	/// Span of the parenthesized arguments.
	pub arguments_span: Span,
	/// Span covering the complete call.
	pub span: Span,
}

/// A single parenthesized grouping expression.
#[derive(Debug, Clone)]
pub struct StyleGroupedValue {
	/// Grouped expression.
	pub expression: Box<StyleValueExpression>,
	/// Span of the parentheses.
	pub span: Span,
}

/// A non-empty sequence or list of value expressions.
#[derive(Debug, Clone)]
pub struct StyleValueCollection {
	/// Items in source order.
	pub items: Vec<StyleValueExpression>,
	/// Spans of authored comma separators.
	pub comma_spans: Vec<Span>,
	/// Span of the surrounding delimiter.
	pub span: Span,
}

/// The only macro-shaped escape accepted by the value parser.
#[derive(Debug, Clone)]
pub struct StyleUncheckedFunction {
	/// Function name inside `unchecked_fn!`.
	pub name: StyleValueName,
	/// Losslessly retained function arguments.
	pub arguments: StyleRawTokenGroup,
	/// Span of the `unchecked_fn` name.
	pub macro_span: Span,
	/// Span of the `!` punctuation.
	pub bang_span: Span,
	/// Span covering the complete unchecked function expression.
	pub span: Span,
}

/// One balanced raw token group retained by the explicit unchecked escape.
#[derive(Debug, Clone)]
pub struct StyleRawTokenGroup {
	/// Delimiter used by the function call.
	pub delimiter: Delimiter,
	/// Token trees in source order, including nested balanced groups.
	pub tokens: Vec<TokenTree>,
	/// Span of the delimiter group.
	pub span: Span,
}
