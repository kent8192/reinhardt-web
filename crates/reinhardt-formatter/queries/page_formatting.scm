; Topiary formatting rules for the Reinhardt page! DSL.

(string) @leaf
(char) @leaf
(raw_string) @leaf
(line_comment) @leaf @append_hardline
(block_comment) @leaf
(fragment) @leaf
(closure_args) @leaf
(component_identifier) @leaf

; === Token spacing ===

; Space between page closure args and the closure body.
(
  (closure_args) @append_space
  .
  (block)
)

; Space between semantic element/component names and their bodies.
(
  [(fragment) (component_identifier)] @append_space
  .
  (block)
)

; Space between semantic attribute/event heads and their Rust expression values.
(attribute
  (fragment) @append_space
  .
  (rustfmt_island)
)

(event_attribute
  (fragment) @append_space
  .
  (rustfmt_island)
)

; Space between Rust closure heads and their block bodies inside values.
(rustfmt_island
  (fragment) @append_space
  .
  (block))

; Commas owned by semantic attributes/events separate following DSL items.
(attribute
  (comma) @append_hardline)

(event_attribute
  (comma) @append_hardline)

; Interpolation blocks stay inline with spaces around the Rust expression.
(interpolation
  "{" @prepend_space
  .
  (rustfmt_island) @prepend_space @append_space
  .
  "}")

; Keep else and else-if clauses attached to the preceding control-flow block.
(if_control_flow
  (block
    "}" @append_space)
  .
  (else_clause))

(else_clause
  (fragment) @append_space
  .
  [(control_flow) (block)])

; Separate adjacent semantic control-flow wrappers.
(_
  (control_flow)
  .
  (control_flow
    (if_control_flow
      (fragment) @prepend_hardline)))

(_
  (control_flow)
  .
  (control_flow
    (for_control_flow
      (fragment) @prepend_hardline)))

(_
  (control_flow)
  .
  (control_flow
    (match_control_flow
      (fragment) @prepend_hardline)))

; Space after fragment ending with : or = before strings/chars/raw strings.
(
  (fragment) @append_space
  .
  [(string) (char) (raw_string)]
  (#match? @append_space "[:=]$")
)

; Space after fragment ending with : or = before parens/brackets/blocks.
(
  (fragment) @append_space
  .
  [(paren) (bracket) (block)]
  (#match? @append_space "[:=]$")
)

; Space between string/char/raw string and following identifier fragment.
(
  [(string) (char) (raw_string)] @append_space
  .
  (fragment) @leaf
  (#match? @leaf "^(r#)?[A-Za-z_][A-Za-z0-9_]*")
)

; Space between string/char/raw string and following semantic element.
(
  [(string) (char) (raw_string)] @append_space
  .
  (element
    (fragment) @leaf)
  (#match? @leaf "^(r#)?[A-Za-z_][A-Za-z0-9_]*")
)

; Space before operator fragment following paren/bracket/string.
(
  [
    (paren)
    (bracket)
    (string)
    (char)
    (raw_string)
  ] @append_space
  .
  (fragment) @leaf
  (#match? @leaf "^(=>|==|!=|<=|>=|&&|\\|\\||=|<|>|\\+|-|\\*|/|%|as\\b)")
)

; Commas in parens/brackets/source get trailing space.
(paren
  (comma) @append_space)

(bracket
  (comma) @append_space)

(source
  (comma) @append_space)

; === Block formatting ===
;
; Block types by child count:
;   empty:       { immediately followed by }
;   single-leaf: { . leaf . }
;   multi-item:  two or more named children after {

; --- Empty blocks: inline {} ---
(block
  "{" @prepend_space
  .
  "}")

; --- Single-leaf blocks: inline { item } ---
(block
  "{" @prepend_space
  .
  [(fragment) (string) (char) (raw_string) (paren) (bracket) (block) (interpolation) (rustfmt_island)] @prepend_space @append_space
  .
  "}")

; --- Single semantic-child blocks: expanded ---
(block
  "{" @prepend_space @append_hardline @append_indent_start
  .
  [(element) (component_call) (control_flow) (attribute) (event_attribute)])

(block
  "{"
  .
  [(element) (component_call) (control_flow) (attribute) (event_attribute)]
  "}" @prepend_hardline @prepend_indent_end)

; --- Multi-item blocks: expanded ---
; Opening brace gets hardline + indent when block has 2+ named children.
(block
  "{" @prepend_space @append_hardline @append_indent_start
  .
  (_)
  .
  (_))

; Closing brace gets hardline + dedent (anchored to { to avoid double-capture).
(block
  "{"
  .
  (_)
  .
  (_)
  "}" @prepend_hardline @prepend_indent_end)

; Commas and semicolons force line breaks within blocks.
(block
  (comma) @append_hardline)

(block
  (semicolon) @append_hardline)

; === Block separation ===

; After closing brace, hardline before non-else items.
(_
  (block
    "}" @append_hardline)
  .
  [(block) (string) (char) (raw_string) (element) (component_call) (control_flow) (attribute) (event_attribute) (interpolation)])

(_
  (block
    "}" @append_hardline)
  .
  [(fragment) (component_identifier)] @leaf
  (#not-match? @leaf "^else\\b"))

; After closing brace, space before else (keeps } else { on one line).
(_
  (block
    "}" @append_space)
  .
  (fragment) @leaf
  (#match? @leaf "^else\\b"))
