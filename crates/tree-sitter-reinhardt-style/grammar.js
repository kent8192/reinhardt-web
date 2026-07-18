module.exports = grammar({
  name: 'reinhardt_style',

  extras: $ => [/[ \t\r\n]+/],

  word: $ => $.identifier,

  conflicts: $ => [
    [$.selector, $.value_expression],
    [$.selector, $.css_name],
  ],

  rules: {
    source: $ => repeat($._item),

    _item: $ => choice(
      $.definition_block,
      $.media_rule,
      $.style_rule,
      $.style_block,
      $.line_comment,
      $.block_comment,
    ),

    definition_block: $ => seq(
      field('kind', choice('globals', 'vars')),
      '{',
      repeat(choice($.typed_declaration, $.line_comment, $.block_comment)),
      '}',
    ),

    typed_declaration: $ => seq(
      field('name', $.identifier),
      ':',
      field('type', $.identifier),
      optional(seq('=', field('default', $.value_expression))),
      ';',
    ),

    style_rule: $ => seq(
      field('selectors', $.selector_list),
      field('body', $.style_block),
    ),

    style_block: $ => seq(
      '{',
      repeat(choice(
		$.definition_block,
        $.property_declaration,
        $.style_rule,
        $.media_rule,
        $.line_comment,
        $.block_comment,
      )),
      '}',
    ),

    media_rule: $ => seq(
      '@',
      'media',
      field('condition', $.media_condition),
      field('body', $.style_block),
    ),

    media_condition: $ => repeat1(choice(
      $.identifier,
      $.signed_number,
      $.number,
      $.string,
      $.raw_string,
      $.paren_group,
      $.bracket_group,
      $.operator,
      ',',
      ':',
    )),

    property_declaration: $ => seq(
      field('name', $.css_name),
      ':',
      field('value', $.value_expression),
      ';',
    ),

    selector_list: $ => seq($.selector, repeat(seq(',', $.selector))),

    selector: $ => repeat1(choice(
      $.identifier,
      $.number,
      $.string,
      $.attribute_selector,
      $.paren_group,
      '.',
      '#',
      '&',
      ':',
      '>',
      '+',
      '~',
      '*',
      '=',
    )),

    attribute_selector: $ => seq(
      '[',
      repeat(choice($.identifier, $.string, $.number, $.operator, '=')),
      ']',
    ),

    value_expression: $ => repeat1($._value_atom),

    _value_atom: $ => choice(
      $.identifier,
      $.signed_number,
      $.number,
      $.hex_color,
      $.string,
      $.raw_string,
      $.paren_group,
      $.bracket_group,
      $.brace_group,
      $.operator,
      '.',
      '::',
      ',',
      '!',
      ':',
      '#',
    ),

    paren_group: $ => seq('(', repeat(choice($._value_atom, $.line_comment, $.block_comment)), ')'),
    bracket_group: $ => seq('[', repeat(choice($._value_atom, $.line_comment, $.block_comment)), ']'),
    brace_group: $ => seq('{', repeat(choice($._value_atom, $.line_comment, $.block_comment)), '}'),

    css_name: $ => $.identifier,
    identifier: _ => /(r#)?[A-Za-z_][A-Za-z0-9_-]*/,
    signed_number: _ => /[+-][0-9]+(\.[0-9]+)?([A-Za-z]+|%)?/,
    number: _ => /[0-9]+(\.[0-9]+)?([A-Za-z]+|%)?/,
    hex_color: _ => /#[0-9A-Fa-f]{3,8}/,
    string: _ => token(seq('"', repeat(choice(/[^"\\]/, /\\./)), '"')),
    raw_string: _ => token(/r#*"[^"\n]*"#*/),
    operator: _ => token(choice('<=', '>=', '==', '!=', '&&', '||', '+', '-', '*', '/', '<', '>')),
    line_comment: _ => token(seq('//', /[^\n]*/)),
    block_comment: _ => token(seq('/*', /([^*]|\*[^/])*/, '*/')),
  },
});
