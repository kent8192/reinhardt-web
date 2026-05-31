; Topiary formatting rules for the Reinhardt form! DSL.

(string) @leaf
(char) @leaf
(raw_string) @leaf
(line_comment) @leaf @append_hardline
(block_comment) @leaf
(fragment) @leaf

; === Token spacing ===

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
  [(fragment) (string) (char) (raw_string) (paren) (bracket) (block)] @prepend_space @append_space
  .
  "}")

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
  [(block) (string) (char) (raw_string)])

(_
  (block
    "}" @append_hardline)
  .
  (fragment) @leaf
  (#not-match? @leaf "^else\\b"))

; After closing brace, space before else (keeps } else { on one line).
(_
  (block
    "}" @append_space)
  .
  (fragment) @leaf
  (#match? @leaf "^else\\b"))
