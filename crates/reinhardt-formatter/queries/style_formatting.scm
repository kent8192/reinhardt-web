; Topiary formatting rules for the Reinhardt style! DSL.

(identifier) @leaf
(signed_number) @leaf
(number) @leaf
(hex_color) @leaf
(string) @leaf
(raw_string) @leaf
(line_comment) @leaf @append_hardline
(block_comment) @leaf
(operator) @leaf @prepend_space @append_space

(definition_block
  "{" @prepend_space @append_hardline @append_indent_start
  "}" @prepend_indent_end @prepend_hardline @append_hardline)

(style_block
  "{" @prepend_space @append_hardline @append_indent_start
  "}" @prepend_indent_end @prepend_hardline)

(typed_declaration
  ":" @append_space
  "=" @prepend_space @append_space
  ";" @append_hardline)

(typed_declaration
  ":" @append_space
  ";" @append_hardline)

(property_declaration
  ":" @append_space
  ";" @append_hardline)

(selector_list
  "," @append_space)

(paren_group
  "," @append_space)

(bracket_group
  "," @append_space)

(selector
  [">" "+" "~"] @prepend_space @append_space)

(paren_group
  ":" @append_space)

(style_rule
  body: (style_block
    "}" @append_hardline))

(media_rule
  "media" @append_space)

(media_rule
  (media_condition) @append_space)
