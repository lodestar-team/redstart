/**
 * Tree-sitter grammar for Redstart.
 *
 * Drives syntax highlighting on GitHub, Neovim, Helix, and Zed. Build with
 * `tree-sitter generate && tree-sitter test` (requires the tree-sitter CLI).
 */

const PREC = {
  or: 1,
  and: 2,
  equality: 3,
  comparison: 4,
  additive: 5,
  multiplicative: 6,
  unary: 7,
  call: 8,
};

module.exports = grammar({
  name: 'redstart',

  word: $ => $.identifier,

  extras: $ => [/\s/, $.line_comment, $.block_comment],

  rules: {
    source_file: $ => repeat($._definition),

    _definition: $ => choice(
      $.mod_declaration,
      $.use_declaration,
      $.abi_declaration,
      $.entity_declaration,
      $.enum_declaration,
      $.source_declaration,
      $.template_declaration,
      $.handler_declaration,
      $.function_declaration,
      $.test_declaration,
    ),

    // ---- declarations ----

    mod_declaration: $ => seq(optional('pub'), 'mod', field('name', $.identifier), ';'),

    use_declaration: $ => seq('use', sep1($.identifier, '::'), ';'),

    abi_declaration: $ => seq(
      'abi', field('name', $.identifier), 'from', field('path', $.string), optional(';'),
    ),

    entity_declaration: $ => seq(
      'entity',
      field('name', $.identifier),
      repeat(field('modifier', $.identifier)),
      '{', repeat($.field_declaration), '}',
    ),

    enum_declaration: $ => seq(
      'enum',
      field('name', $.identifier),
      '{', sepTrailing(field('variant', $.identifier), ','), '}',
    ),

    field_declaration: $ => seq(
      field('name', $.identifier),
      ':',
      field('type', $._type),
      optional(seq('derived', 'from', field('back', $.identifier))),
      optional(','),
    ),

    source_declaration: $ => seq(
      'source', field('name', $.identifier), '{', repeat($.setting), '}',
    ),

    template_declaration: $ => seq(
      'template', field('name', $.identifier), '{', repeat($.setting), '}',
    ),

    setting: $ => seq(
      field('key', $.identifier), ':', field('value', $._expression), optional(','),
    ),

    handler_declaration: $ => seq(
      'handler', 'on',
      field('source', $.identifier), '.', field('event', $.identifier),
      '(', field('param', $.identifier), ')',
      field('body', $.block),
    ),

    function_declaration: $ => seq(
      optional('pub'), 'fn',
      field('name', $.identifier),
      '(', sepTrailing($.parameter, ','), ')',
      optional(seq('->', field('return_type', $._type))),
      field('body', $.block),
    ),

    parameter: $ => seq(field('name', $.identifier), ':', field('type', $._type)),

    test_declaration: $ => seq('test', field('name', $.string), field('body', $.block)),

    // ---- types ----

    _type: $ => choice($.list_type, $.generic_type, $.type_path),

    list_type: $ => seq('[', $._type, ']'),

    generic_type: $ => seq(
      field('base', $.type_path), '<', sep1($._type, ','), '>',
    ),

    type_path: $ => sep1($.type_identifier, '::'),

    // ---- statements ----

    block: $ => seq('{', repeat($._statement), '}'),

    _statement: $ => choice(
      $.let_statement,
      $.assignment_statement,
      $.return_statement,
      $.if_statement,
      $.while_statement,
      $.for_statement,
      $.expression_statement,
    ),

    if_statement: $ => seq(
      'if', field('condition', $._expression), field('consequence', $.block),
      repeat(seq('else', 'if', field('condition', $._expression), field('consequence', $.block))),
      optional(seq('else', field('alternative', $.block))),
    ),

    while_statement: $ => seq(
      'while', field('condition', $._expression), field('body', $.block),
    ),

    for_statement: $ => seq(
      'for', field('variable', $.identifier), 'in',
      field('iterable', choice($._expression, $.range)),
      field('body', $.block),
    ),

    range: $ => prec.left(seq(field('start', $._expression), '..', field('end', $._expression))),

    let_statement: $ => seq(
      'let', field('name', $.identifier),
      optional(seq(':', field('type', $._type))),
      '=', field('value', $._expression),
      optional(';'),
    ),

    assignment_statement: $ => prec(-1, seq(
      field('target', $._expression), '=', field('value', $._expression), optional(';'),
    )),

    return_statement: $ => seq('return', optional($._expression), optional(';')),

    expression_statement: $ => seq($._expression, optional(';')),

    // ---- expressions ----

    _expression: $ => choice(
      $.integer,
      $.hex,
      $.decimal,
      $.string,
      $.boolean,
      $.path,
      $.field_expression,
      $.call_expression,
      $.index_expression,
      $.array,
      $.record,
      $.unary_expression,
      $.binary_expression,
      $.match_expression,
      $.parenthesized_expression,
    ),

    parenthesized_expression: $ => seq('(', $._expression, ')'),

    field_expression: $ => prec(PREC.call, seq(
      field('base', $._expression), '.', field('field', $.identifier),
    )),

    call_expression: $ => prec(PREC.call, seq(
      field('function', $._expression),
      '(', sepTrailing($._expression, ','), ')',
    )),

    index_expression: $ => prec(PREC.call, seq(
      field('base', $._expression), '[', field('index', $._expression), ']',
    )),

    array: $ => seq('[', sepTrailing($._expression, ','), ']'),

    record: $ => seq('{', sepTrailing($.record_field, ','), '}'),

    record_field: $ => seq(field('name', $.identifier), ':', field('value', $._expression)),

    unary_expression: $ => prec(PREC.unary, seq(choice('!', '-'), $._expression)),

    binary_expression: $ => {
      const table = [
        ['||', PREC.or],
        ['&&', PREC.and],
        ['==', PREC.equality],
        ['!=', PREC.equality],
        ['<', PREC.comparison],
        ['<=', PREC.comparison],
        ['>', PREC.comparison],
        ['>=', PREC.comparison],
        ['+', PREC.additive],
        ['-', PREC.additive],
        ['*', PREC.multiplicative],
        ['/', PREC.multiplicative],
        ['%', PREC.multiplicative],
      ];
      return choice(...table.map(([op, p]) => prec.left(p, seq(
        field('left', $._expression),
        field('operator', op),
        field('right', $._expression),
      ))));
    },

    match_expression: $ => seq(
      'match', field('value', $._expression),
      '{', repeat($.match_arm), '}',
    ),

    match_arm: $ => seq(
      field('pattern', $.pattern), '=>', field('body', $.block), optional(','),
    ),

    pattern: $ => choice(
      seq(field('ctor', $.identifier), '(', sep1($.identifier, ','), ')'),
      field('ctor', $.identifier),
      '_',
    ),

    path: $ => sep1($.identifier, '::'),

    // ---- terminals ----

    identifier: _ => /[A-Za-z_][A-Za-z0-9_]*/,
    type_identifier: _ => /[A-Za-z_][A-Za-z0-9_]*/,

    integer: _ => /[0-9][0-9_]*/,
    hex: _ => /0x[0-9a-fA-F]+/,
    decimal: _ => /[0-9][0-9_]*\.[0-9][0-9_]*/,
    string: _ => /"([^"\\]|\\.)*"/,
    boolean: _ => choice('true', 'false'),

    line_comment: _ => token(seq('//', /[^\n]*/)),
    block_comment: _ => token(seq('/*', /([^*]|\*[^/])*/, '*/')),
  },
});

function sep1(rule, separator) {
  return seq(rule, repeat(seq(separator, rule)));
}

function sepTrailing(rule, separator) {
  return optional(seq(rule, repeat(seq(separator, rule)), optional(separator)));
}
