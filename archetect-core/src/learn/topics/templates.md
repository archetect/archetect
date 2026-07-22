# templates — ATL: Jinja-shaped, Lua underneath

Template files render with ATL (Archetect Template Language): `{{ expression | filter }}`
interpolation and `{% lua statements %}` logic blocks. It COMPILES TO LUA — inside `{% %}`
you write real Lua against the context table, not a template mini-language:

```
{% if persistence == "Postgres" then %}
pub mod repository;
{% end %}
{% for _, field in ipairs(fields) do %}
    pub {{ field.name | snake_case }}: {{ field.ty }},
{% end %}
```

- Both file CONTENT and file/directory NAMES render: a file named
  `{{ service_name | snake_case }}.rs` lands under the rendered name.
- `{% include "partials/header" %}` inlines at compile time (sandboxed to the archetype;
  the archetype's `includes/` dir is on the path).
- Every filter is also a function: `{{ x | trim }}` ≡ `{{ trim(x) }}`.

## The filter/function set (shapes: `archetect introspect <name>`)

| Family | Names |
|---|---|
| casing/inflection | `snake_case pascal_case camel_case kebab_case train_case constant_case class_case title_case sentence_case package_case directory_case cobol_case lower upper pluralize singularize ordinalize deordinalize` |
| strings | `default truncate replace trim trim_start trim_end indent string_repeat split length concat` |
| collections | `join first last sort reverse contains unique` |
| datetime | `now now_utc today year timestamp date` |
| paths | `path_join basename dirname extname path_normalize` |
| ids | `uuid uuid_nil` |

Custom filters are Lua, registered from the script: `template.register_filters{ shout =
function(s) return s:upper() .. "!" end }` → `{{ name | shout }}`.

## Modes (manifest `templating:` — `archetect learn manifest`)

- `undefined: strict` — an undefined `{{ var }}` is a render ERROR instead of empty output.
  Turn it on; it catches typo'd keys at render time instead of shipping blank spots.
- `trim_blocks` / `lstrip_blocks` — newline/indent hygiene around `{% %}` tags.

## From the script

`directory.render("content/base", ctx)` renders a whole tree; `file.render(src, ctx, opts)`
one file; `template.render("{{ x }}", ctx)` a string — probe filters without an archetype:
`archetect eval 'local c = Context.new() c:set("x", "foo bar") return
template.render("{{ x | train_case }}", c)'`. Overwrite behavior is the `Existing.*` policy
each call can set.

Go deeper: `archetect learn cases` (the casing filters' other home) · `archetect learn
authoring` (who calls render).
