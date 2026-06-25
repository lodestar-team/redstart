; Highlight queries for Redstart.

; Keywords
[
  "mod" "use" "abi" "from" "entity" "enum" "interface" "implements" "source" "template"
  "handler" "on" "call" "block" "file" "every" "once" "fn" "test" "derived" "pub"
] @keyword

"let" @keyword
"return" @keyword
"match" @keyword

[
  "if" "else" "while" "for" "in"
] @keyword.control

; Declaration names
(abi_declaration name: (identifier) @type)
(entity_declaration name: (identifier) @type)
(enum_declaration name: (identifier) @type)
(enum_declaration variant: (identifier) @constant)
(interface_declaration name: (identifier) @type)
(entity_declaration interface: (identifier) @type)
(source_declaration name: (identifier) @type)
(template_declaration name: (identifier) @type)
(function_declaration name: (identifier) @function)
(handler_declaration event: (identifier) @function)

; Fields and settings
(field_declaration name: (identifier) @property)
(setting key: (identifier) @property)
(record_field name: (identifier) @property)
(field_expression field: (identifier) @property)

; Types
(type_identifier) @type
(generic_type base: (type_path) @type)

; Parameters
(parameter name: (identifier) @variable.parameter)
(handler_declaration param: (identifier) @variable.parameter)

; Patterns
(pattern ctor: (identifier) @constructor)

; Literals
(integer) @number
(hex) @number
(decimal) @number
(string) @string
(boolean) @constant.builtin

; Operators & punctuation
[ "+" "-" "*" "/" "%" "==" "!=" "<" "<=" ">" ">=" "&&" "||" "!" "=" "=>" "->" ] @operator
[ "{" "}" "(" ")" "[" "]" ] @punctuation.bracket
[ ":" ";" "," "." "::" ] @punctuation.delimiter

; Comments
[ (line_comment) (block_comment) ] @comment

; Calls
(call_expression function: (path (identifier) @function.call))
