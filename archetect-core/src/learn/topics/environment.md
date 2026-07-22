# environment — this machine's archetect, computed now

Configuration merges, most-specific last: built-in defaults → user config
(`~/.config/archetect/config.yaml`) → project `.archetect.yaml` (walked up from the working
directory) → `-c/--config-file` → flags/env. `archetect config merged` prints the result;
`archetect config defaults` prints a scaffold; `archetect system layout` shows every path
(config, cache, data).

## Here, right now

[[slot:catalog_tree]]

[[slot:project_config]]

[[slot:locals]]

[[slot:cache_state]]

[[slot:annotations]]

## The flags-over-config rule

Every boolean in config has a per-run flag override (`--offline`, `--headless`, `--local`,
`--allow-exec`, `--force-update`, `--dry-run`) and most have an `ARCHETECT_*` env twin. Flags
win. Switches overlay: `-s name` adds, `-s name=false` removes an inherited one — the same
semantics at every layer (config → entry → CLI); see `archetect learn rendering`.

Go deeper: `archetect learn catalogs` (what the catalog is) · `archetect learn sources`
(locals + cache mechanics).
