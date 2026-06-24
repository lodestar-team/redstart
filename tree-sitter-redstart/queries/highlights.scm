; Highlight queries for Redstart.

; Keywords
[
  "mod" "use" "abi" "from" "entity" "source" "template"
  "handler" "on" "fn" "test" "derived" "pub"
] @keyword

"let" @keyword
"return" @keyword
"match" @keyword

; Declaration names
(abi_declaration name: (identifier) @type)
(entity_declaration name: (identifier) @type)
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
