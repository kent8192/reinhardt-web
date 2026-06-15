module.exports = grammar({
  name: 'reinhardt_page',

  externals: $ => [
    $.line_comment,
    $.block_comment,
    $.raw_string,
    $._closure_args,
    $._if_head,
    $._for_head,
    $._match_head,
    $._else_head,
    $._attribute_head,
    $._event_attribute_head,
    $.fragment,
  ],

  extras: $ => [/[ \t\r\n]+/],

  conflicts: $ => [
    [$._item, $._rust_item],
    [$._item, $._rust_block_item],
    [$.block, $._rust_block],
    [$.rustfmt_island, $._rust_block_item],
  ],

  rules: {
    source: $ => repeat(choice($.page_closure, $._item)),

    _item: $ => choice(
      $.control_flow,
      $.event_attribute,
      $.attribute,
      $.component_call,
      $.element,
      $.interpolation,
      $.block,
      $.paren,
      $.bracket,
      $.line_comment,
      $.block_comment,
      $.raw_string,
      $.string,
      $.char,
      $.comma,
      $.semicolon,
      $.fragment,
    ),

    page_closure: $ => prec.right(seq(
      $.closure_args,
      $.block,
    )),

    closure_args: $ => $._closure_args,

    control_flow: $ => choice(
      $.if_control_flow,
      $.for_control_flow,
      $.match_control_flow,
    ),

    if_control_flow: $ => seq(
      alias($._if_head, $.fragment),
      $.block,
      optional($.else_clause),
    ),

    else_clause: $ => seq(
      alias($._else_head, $.fragment),
      choice($.control_flow, $.block),
    ),

    for_control_flow: $ => seq(
      alias($._for_head, $.fragment),
      $.block,
    ),

    match_control_flow: $ => seq(
      alias($._match_head, $.fragment),
      $.block,
    ),

    event_attribute: $ => prec.right(3, seq(
      alias($._event_attribute_head, $.fragment),
      $.rustfmt_island,
      optional($.comma),
    )),

    attribute: $ => prec.right(2, seq(
      alias($._attribute_head, $.fragment),
      $.rustfmt_island,
      optional($.comma),
    )),

    component_call: $ => prec.right(seq(
      $.component_identifier,
      optional($.paren),
      optional($.block),
    )),

    element: $ => prec(1, seq(
      $.fragment,
      $.block,
    )),

    interpolation: $ => prec(1, seq(
      '{',
      $.rustfmt_island,
      '}',
    )),

    block: $ => seq('{', repeat($._item), '}'),
    paren: $ => seq('(', repeat($._item), ')'),
    bracket: $ => seq('[', repeat($._item), ']'),

    rustfmt_island: $ => prec.right(repeat1($._rust_item)),

    _rust_item: $ => choice(
      alias($._rust_block, $.block),
      $.paren,
      $.bracket,
      $.line_comment,
      $.block_comment,
      $.raw_string,
      $.string,
      $.char,
      $.fragment,
    ),

    _rust_block: $ => seq('{', repeat($._rust_block_item), '}'),

    _rust_block_item: $ => choice(
      $._rust_item,
      $.comma,
      $.semicolon,
    ),

    component_identifier: $ => token(prec(1, /[A-Z][A-Za-z0-9_]*/)),

    string: $ => token(seq('"', repeat(choice(/[^"\\]/, /\\./)), '"')),
    char: $ => token(seq("'", repeat(choice(/[^'\\]/, /\\./)), "'")),
    comma: $ => ',',
    semicolon: $ => ';',
  },
});
