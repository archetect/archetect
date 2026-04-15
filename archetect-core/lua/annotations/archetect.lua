---@meta archetect
--- Archetect v3 Lua API — Type annotations for LuaLS
--- Place this file in your workspace or configure LuaLS to find it
--- for full autocomplete and hover documentation.

--
-- Context
--

---@class Context
---A smart map that holds template variables and provides prompt methods.
---Create with `Context.new()`. Supports `get`, `set`, `has`, and all prompt types.
---When a prompt includes `cases`, the Context automatically expands the value
---into multiple case-variant entries (e.g., snake_case, PascalCase, etc.).
Context = {}

---Create a new Context.
---Pre-loaded answers from the CLI (`-a key=value`) are available immediately.
---@return Context
function Context.new() end

---Get a value from the context.
---@param key string Key to retrieve
---@return any value The stored value, or nil if not present
function Context:get(key) end

---Check if a key exists in the context.
---@param key string
---@return boolean
function Context:has(key) end

---Check if an array stored at `key` contains the given value.
---Returns false if the key doesn't exist or isn't an array.
---@param key string Key of the array to search
---@param value string Value to look for
---@return boolean
function Context:contains(key, value) end

---Set a value directly in the context.
---@param key string Key to store under
---@param value any Value to store
---@param opts? {cases?: CaseSpec|CaseSpec[]} Optional case expansion
function Context:set(key, value, opts) end

---Prompt for text input, store in context, and return the value.
---Returns the user's raw input; use `ctx:get(key)` to read case-expanded
---variants produced by `opts.cases`. Returns `nil` when an optional
---prompt is skipped.
---@param message string Prompt message displayed to the user
---@param key string Key to store the result under
---@param opts? TextPromptOpts
---@return string? value
function Context:prompt_text(message, key, opts) end

---Prompt for integer input, store in context, and return the value.
---Returns `nil` when an optional prompt is skipped.
---@param message string Prompt message
---@param key string Key to store the result under
---@param opts? IntPromptOpts
---@return integer? value
function Context:prompt_int(message, key, opts) end

---Prompt for boolean confirmation, store in context, and return the value.
---Returns `nil` when an optional prompt is skipped.
---@param message string Prompt message
---@param key string Key to store the result under
---@param opts? ConfirmPromptOpts
---@return boolean? value
function Context:prompt_confirm(message, key, opts) end

---Prompt for selection from a list, store in context, and return the value.
---Returns the selected string; use `ctx:get(key)` for case-expanded variants.
---Returns `nil` when an optional prompt is skipped.
---@param message string Prompt message
---@param key string Key to store the result under
---@param options string[] Available options
---@param opts? SelectPromptOpts
---@return string? value
function Context:prompt_select(message, key, options, opts) end

---Prompt for multiple selections, store in context, and return the list.
---Returns `nil` when an optional prompt is skipped.
---@param message string Prompt message
---@param key string Key to store the result under
---@param options string[] Available options
---@param opts? MultiSelectPromptOpts
---@return string[]? value
function Context:prompt_multiselect(message, key, options, opts) end

---Prompt for a list of strings, store in context, and return the list.
---Returns `nil` when an optional prompt is skipped.
---@param message string Prompt message
---@param key string Key to store the result under
---@param opts? ListPromptOpts
---@return string[]? value
function Context:prompt_list(message, key, opts) end

---Prompt for text via editor, store in context, and return the value.
---Returns `nil` when an optional prompt is skipped.
---@param message string Prompt message
---@param key string Key to store the result under
---@param opts? EditorPromptOpts
---@return string? value
function Context:prompt_editor(message, key, opts) end

---@class TextPromptOpts
---@field default? string Default value
---@field help? string Help text shown to the user
---@field placeholder? string Placeholder text
---@field min? integer Minimum length
---@field max? integer Maximum length
---@field optional? boolean Whether the prompt can be skipped
---@field cases? CaseSpec|CaseSpec[] Case expansion rules
---@field answer_key? string Alternate key to look up pre-supplied answers

---@class IntPromptOpts
---@field default? integer Default value
---@field help? string Help text
---@field placeholder? string Placeholder text
---@field min? integer Minimum value
---@field max? integer Maximum value
---@field optional? boolean Whether the prompt can be skipped
---@field answer_key? string Alternate key to look up pre-supplied answers

---@class ConfirmPromptOpts
---@field default? boolean Default value
---@field help? string Help text
---@field placeholder? string Placeholder text
---@field optional? boolean Whether the prompt can be skipped
---@field answer_key? string Alternate key to look up pre-supplied answers

---@class SelectPromptOpts
---@field default? string Default selection (may be an off-list value when allow_other = true)
---@field help? string Help text
---@field placeholder? string Placeholder text
---@field optional? boolean Whether the prompt can be skipped
---@field allow_other? boolean Append an "Other..." entry that opens a free-text prompt
---@field other_label? string Label for the "other" entry (default: "Other...")
---@field cases? CaseSpec|CaseSpec[] Case expansion rules
---@field answer_key? string Alternate key to look up pre-supplied answers

---@class MultiSelectPromptOpts
---@field default? string[] Default selections
---@field help? string Help text
---@field placeholder? string Placeholder text
---@field min? integer Minimum selections
---@field max? integer Maximum selections
---@field optional? boolean Whether the prompt can be skipped
---@field answer_key? string Alternate key to look up pre-supplied answers

---@class ListPromptOpts
---@field help? string Help text
---@field placeholder? string Placeholder text
---@field min? integer Minimum items
---@field max? integer Maximum items
---@field optional? boolean Whether the prompt can be skipped
---@field answer_key? string Alternate key to look up pre-supplied answers

---@class EditorPromptOpts
---@field default? string Default text in editor
---@field help? string Help text
---@field placeholder? string Placeholder text
---@field answer_key? string Alternate key to look up pre-supplied answers

--
-- Cases
--

---@class CaseSpec
---Case expansion specification. Created by `Cases.*` functions.

---@class CaseStyle
---A case transformation style. Use `Case.*` constants (e.g., `Case.Snake`, `Case.Pascal`).
---Can also transform strings directly via `:apply(input)`.

---Apply this case transformation to a string.
---@param input string The string to transform
---@return string result The transformed string
function CaseStyle:apply(input) end

---@class Case
---Case style constants. Use these instead of string names for type safety.
---@field Snake CaseStyle snake_case
---@field Pascal CaseStyle PascalCase
---@field Camel CaseStyle camelCase
---@field Kebab CaseStyle kebab-case
---@field Train CaseStyle Train-Case
---@field Constant CaseStyle CONSTANT_CASE
---@field Title CaseStyle Title Case
---@field Lower CaseStyle lowercase
---@field Upper CaseStyle UPPERCASE
---@field Sentence CaseStyle Sentence case
---@field Package CaseStyle package.case
---@field Directory CaseStyle directory/case
---@field Cobol CaseStyle COBOL-CASE
---@field Plural CaseStyle Pluralized
---@field Singular CaseStyle Singularized
Case = {}

---@class Cases
---Case expansion presets and constructors.
Cases = {}

---Standard programming cases: snake_case, PascalCase, camelCase, kebab-case, Train-Case, CONSTANT_CASE.
---@return CaseSpec
function Cases.programming() end

---All available case variants including title, sentence, package, directory, cobol.
---@return CaseSpec
function Cases.all() end

---Custom set of specific case styles.
---@param ... CaseStyle Case styles (e.g., `Case.Snake, Case.Pascal`)
---@return CaseSpec
function Cases.set(...) end

---Fixed key with a specific case style applied to the value.
---Use for cases where the key name can't match the value shape (e.g., title case).
---@param key string The exact key name to use
---@param style CaseStyle The case style (e.g., `Case.Title`)
---@return CaseSpec
function Cases.fixed(key, style) end

--
-- archetect (binary introspection)
--

---@class archetect
---Archetect binary version and raw answers access.
---@field version string Full version string (e.g., "3.0.0")
---@field version_major integer Major version number
---@field version_minor integer Minor version number
---@field version_patch integer Patch version number
archetect = {}

---Get the raw answers table from CLI/YAML/parent archetype.
---Returns a fresh table each call. Use for advanced patterns where
---you need to inspect answers independently of the Context.
---@return table answers Key-value pairs
function archetect.answers() end

--
-- archetype (current archetype introspection)
--

---@class archetype
---Information about the currently executing archetype.
---@field description string Archetype description from manifest
---@field directory string Root directory path of the archetype
---@field authors string[] Author list from manifest
archetype = {}

--
-- component (child archetype rendering)
--

---@class component
---Render child archetype components declared in `archetype.yaml`.
component = {}

---Render a named child archetype component.
---The component must be declared in the `components` section of `archetype.yaml`.
---@param name string Component name from archetype.yaml
---@param context Context Template context
---@param opts? ComponentRenderOpts
function component.render(name, context, opts) end

---@class ComponentRenderOpts
---@field destination? string Subdirectory to render into (relative to current destination)
---@field switches? string[] Switches to pass to child archetype
---@field use_defaults? string[] Keys to use defaults for in child archetype
---@field use_defaults_all? boolean Use defaults for all prompts in child archetype

--
-- directory
--

---@class directory
---Render content directories from the archetype's content directory.
directory = {}

---Render a content directory using the context for template variables.
---@param path string Directory path relative to the archetype's content root
---@param context Context Template context
---@param opts? DirectoryRenderOpts
function directory.render(path, context, opts) end

---@class DirectoryRenderOpts
---@field destination? string Subdirectory to render into (relative to current destination)
---@field if_exists? ExistingPolicy How to handle existing files (e.g., `Existing.Overwrite`)

--
-- file (single-file counterparts to directory.*)
--

---@class file
---Single-file helpers. Paths resolve against the archetype root by
---default; pass `{ scope = "cwd" }` to resolve against the invocation
---working directory instead. Absolute paths, `..` traversal, and
---`~` expansion are rejected regardless of scope.
file = {}

---Check whether a file exists at the given path.
---@param path string Relative path
---@param opts? FileScopeOpts
---@return boolean exists
function file.exists(path, opts) end

---Read the contents of a file as a string. Errors if the path does
---not exist or is not a regular file. Combine with `format.from_yaml`
---/ `from_json` / `from_toml` to deserialize, then `context:merge(...)`
---to fold into the current context.
---@param path string Relative path
---@param opts? FileScopeOpts
---@return string contents
function file.read(path, opts) end

---Render a single template file from the archetype root to destination.
---Source always resolves against the archetype root (no `scope` here —
---rendering from the caller's cwd would be a footgun). By default the
---destination mirrors the source-relative path; override via
---`opts.destination`.
---@param path string Source path relative to the archetype root
---@param context Context Template context
---@param opts? FileRenderOpts
function file.render(path, context, opts) end

---@class FileScopeOpts
---@field scope? "archetype"|"cwd" Default: `"archetype"`

---@class FileRenderOpts
---@field destination? string Destination path (relative to render destination). Defaults to the source path.
---@field if_exists? ExistingPolicy How to handle existing files (e.g., `Existing.Overwrite`)

--
-- format
--

---@class format
---Serialize values to structured text formats. Accepts Context or plain tables.
format = {}

---Serialize a value to pretty-printed JSON.
---@param value Context|table The value to serialize
---@return string json The JSON string
function format.json(value) end

---Serialize a value to YAML.
---@param value Context|table The value to serialize
---@return string yaml The YAML string
function format.yaml(value) end

---Serialize a value to TOML.
---@param value Context|table The value to serialize (must be a table/map at top level)
---@return string toml The TOML string
function format.toml(value) end

--
-- runtime
--

---@class runtime
---Runtime state of the archetect process.
---@field is_offline boolean Whether archetect is running in offline mode
---@field is_headless boolean Whether archetect is running in headless mode (no interactive prompts)
---@field locals_enabled boolean Whether local directory overrides are enabled
runtime = {}

--
-- exit
--

---Cleanly terminate the script. Does not produce an error — signals
---successful completion to the IO channel. Use for early termination
---when further processing is not needed.
function exit() end

--
-- env
--

---@class env
---Host environment information.
---@field os string Operating system: "macos", "linux", "windows", etc.
---@field arch string CPU architecture: "aarch64", "x86_64", etc.
---@field family string OS family: "unix" or "windows"
---@field is_unix boolean True if running on a Unix-like OS
---@field is_windows boolean True if running on Windows
---@field is_macos boolean True if running on macOS
env = {}

--
-- switches
--

---@class switches
---Query CLI switches passed via `--switch name`.
switches = {}

---Check if a switch is enabled.
---@param name string Switch name
---@return boolean
function switches.is_enabled(name) end

--
-- Existing (enum constants)
--

---@class ExistingPolicy
---Strategy for handling files that already exist at the destination.

---@class Existing
---Policies for handling existing files during rendering.
---@field Overwrite ExistingPolicy Replace existing files
---@field Preserve ExistingPolicy Keep existing files unchanged
---@field Prompt ExistingPolicy Ask the user what to do
Existing = {}

--
-- template
--

---@class template
---Inline Jinja2 template rendering.
template = {}

---Render an inline Jinja2 template string using the context's variables.
---@param tmpl string Template string (e.g., "{{ project_name | title_case }}")
---@param context Context Template context
---@return string result The rendered string
function template.render(tmpl, context) end

--
-- log
--

---@class log
---Structured logging. Messages go to the logging system, not directly to the user.
log = {}

---@param message string
function log.info(message) end
---@param message string
function log.debug(message) end
---@param message string
function log.warn(message) end
---@param message string
function log.error(message) end
---@param message string
function log.trace(message) end

--
-- output
--

---@class output
---User-facing output. Always displayed regardless of log level.
output = {}

---Print a message to the user (stdout).
---@param message string
function output.print(message) end

---Display a prominent banner message (stderr).
---@param message string
function output.banner(message) end
