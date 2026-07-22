# cases — one input, every casing, no reformatting by hand

Casing is archetect's signature move: prompt once for `"customer order"`, and templates can
say `{{ CustomerOrder }}`, `{{ customer_order }}`, `{{ customer-order }}` — because the
Context expanded the input into case-variant keys at prompt time.

```lua
context:prompt_text("Entity Name:", "entity", { cases = Cases.programming() })
-- stores: entity ("customer order"), plus EntityName-style variants per case:
-- entity_name? no — keys are derived from the VALUE's case style applied to the KEY:
-- snake `customer_order`, pascal `CustomerOrder`, camel `customerOrder`,
-- kebab `customer-order`, train `Customer-Order`, constant `CUSTOMER_ORDER` …
```

## The specs

- `Cases.programming()` — snake/pascal/camel/kebab/train/constant. The everyday set.
- `Cases.all()` — all 13 automatic styles (adds title/lower/upper/sentence/package/directory/
  cobol).
- `Cases.set(Case.Snake, Case.Pascal, …)` — exactly these.
- `Cases.fixed("EntityName", Case.Pascal)` — ONE extra key with a fixed transform.
- `Cases.input("entity_raw")` — preserve the untransformed input under an extra key.
- Manual: any `Case.X:apply(str)` — `Case.Plural:apply("calf")` → `"calves"`.

`Case.*` constants: Snake, Pascal, Camel, Kebab, Train, Constant, Title, Lower, Upper,
Sentence, Package, Directory, Cobol, Plural, Singular.

## In templates (ATL filters — the same engine, callable both ways)

```
{{ entity | pascal_case }}     {{ pascal_case(entity) }}
{{ entity | pluralize }}       {{ entity | snake_case | upper_case }}
```

Filter family: `snake_case pascal_case camel_case kebab_case train_case constant_case
class_case title_case sentence_case package_case directory_case cobol_case lower upper
lower_case upper_case pluralize/plural singularize/singular ordinalize deordinalize`.
Inflections are dictionary-smart (soliloquy→soliloquies, calf→calves).

## Choosing where to case

Prefer `cases = …` at the PROMPT (one input, all variants, `interface:`-friendly) over
filters in templates for identifiers used many times; use filters for one-off spots.
Shapes: `archetect introspect Cases`.

Go deeper: `archetect learn templates` (the filter grammar) · `archetect learn prompts`.
