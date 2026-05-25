; Topiary formatting rules for the Reinhardt head! DSL.

(string) @leaf
(char) @leaf
(raw_string) @leaf
(line_comment) @leaf @append_hardline
(block_comment) @leaf
(fragment) @leaf

; Preserve Rust-like token spacing across DSL grammar boundaries.
(
  (fragment) @append_space
  .
  [(string) (char) (raw_string)]
  (#match? @append_space "[:=]$")
)

(
  [(string) (char) (raw_string)] @append_space
  .
  (fragment) @leaf
  (#match? @leaf "^(r#)?[A-Za-z_][A-Za-z0-9_]*")
)

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

(paren
  (comma) @append_space)

(bracket
  (comma) @append_space)

(source
  (comma) @append_space)

(block
  "{" @prepend_space @append_hardline @append_indent_start)

(block
  "}" @prepend_hardline @prepend_indent_end)

(_
  (block
    "}" @append_hardline)
  .
  [(fragment) (block) (string) (char) (raw_string)])

(block
  (comma) @append_hardline)

(block
  (semicolon) @append_hardline)
