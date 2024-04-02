# Archetect Templating Engine

This crate is a vendoring of [cruet](https://github.com/chrislearn/cruet) inflections library.

This vendored version of cruet __is inferior__ to the actual cruet library, and is intended strictly for use within 
Archetect based on its specific requirements.

## Vendoring Justification

`cruet`/`inflector` handles numbers in strings differently based on the case
output.  If you have a string `example3`, camel casing this string would result in `example3`, treating the '3' as part
of the entire word, where-as kebab casing this string would result in `example-3`, treating the '3' as the start of a 
new word.

Archetect is first and for most a code generator. Therefore, it is advantageous for all casing strategies to be
consistent in the casing rules to minimize surprises and ensure generated code is more likely to produce compiling
projects.