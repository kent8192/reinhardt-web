module.exports = grammar({
  name: 'reinhardt_head',

  externals: $ => [
    $.line_comment,
    $.block_comment,
    $.raw_string,
    $.fragment,
  ],

  extras: $ => [/[ \t\r\n]+/],

  rules: {
    source: $ => repeat($._item),

    _item: $ => choice(
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

    block: $ => seq('{', repeat($._item), '}'),
    paren: $ => seq('(', repeat($._item), ')'),
    bracket: $ => seq('[', repeat($._item), ']'),

    string: $ => token(seq('"', repeat(choice(/[^"\\]/, /\\./)), '"')),
    char: $ => token(seq("'", repeat(choice(/[^'\\]/, /\\./)), "'")),
    comma: $ => ',',
    semicolon: $ => ';',
  },
});
