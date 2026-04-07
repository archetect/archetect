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

---Prompt for text input and store in context.
---@param message string Prompt message displayed to the user
---@param key string Key to store the result under
---@param opts? TextPromptOpts
function Context:prompt_text(message, key, opts) end

---Prompt for integer input and store in context.
---@param message string Prompt message
---@param key string Key to store the result under
---@param opts? IntPromptOpts
function Context:prompt_int(message, key, opts) end

---Prompt for boolean confirmation and store in context.
---@param message string Prompt message
---@param key string Key to store the result under
---@param opts? ConfirmPromptOpts
function Context:prompt_confirm(message, key, opts) end

---Prompt for selection from a list and store in context.
---@param message string Prompt message
---@param key string Key to store the result under
---@param options string[] Available options
---@param opts? SelectPromptOpts
function Context:prompt_select(message, key, options, opts) end

---Prompt for multiple selections and store in context.
---@param message string Prompt message
---@param key string Key to store the result under
---@param options string[] Available options
---@param opts? MultiSelectPromptOpts
function Context:prompt_multi_select(message, key, options, opts) end

---Prompt for a list of strings and store in context.
---@param message string Prompt message
---@param key string Key to store the result under
---@param opts? ListPromptOpts
function Context:prompt_list(message, key, opts) end

---Prompt for text via editor and store in context.
---@param message string Prompt message
---@param key string Key to store the result under
---@param opts? EditorPromptOpts
function Context:prompt_editor(message, key, opts) end

---@class TextPromptOpts
---@field default? string Default value
---@field help? string Help text shown to the user
---@field placeholder? string Placeholder text
---@field min? integer Minimum length
---@field max? integer Maximum length
---@field optional? boolean Whether the prompt can be skipped
---@field cases? CaseSpec|CaseSpec[] Case expansion rules

---@class IntPromptOpts
---@field default? integer Default value
---@field help? string Help text
---@field placeholder? string Placeholder text
---@field min? integer Minimum value
---@field max? integer Maximum value
---@field optional? boolean Whether the prompt can be skipped

---@class ConfirmPromptOpts
---@field default? boolean Default value
---@field help? string Help text
---@field placeholder? string Placeholder text
---@field optional? boolean Whether the prompt can be skipped

---@class SelectPromptOpts
---@field default? string Default selection
---@field help? string Help text
---@field placeholder? string Placeholder text
---@field optional? boolean Whether the prompt can be skipped
---@field cases? CaseSpec|CaseSpec[] Case expansion rules

---@class MultiSelectPromptOpts
---@field default? string[] Default selections
---@field help? string Help text
---@field placeholder? string Placeholder text
---@field min? integer Minimum selections
---@field max? integer Maximum selections
---@field optional? boolean Whether the prompt can be skipped

---@class ListPromptOpts
---@field help? string Help text
---@field placeholder? string Placeholder text
---@field min? integer Minimum items
---@field max? integer Maximum items
---@field optional? boolean Whether the prompt can be skipped

---@class EditorPromptOpts
---@field default? string Default text in editor
---@field help? string Help text
---@field placeholder? string Placeholder text

--
-- Cases
--

---@class CaseSpec
---Case expansion specification. Created by `Cases.*` functions.

---@class CaseStyle
---A case transformation style. Use `Case.*` constants (e.g., `Case.Snake`, `Case.Pascal`).

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
-- archetype
--

---@class archetype
---The current archetype's rendering and configuration surface.
archetype = {}

---Render a content directory or a child archetype component.
---
---When `target` is a **string**, renders the named content directory
---(relative to the archetype root) using the context for template variables.
---
---When `target` is an **Archetype** reference (from `Archetype("name")`),
---renders the registered child component with the given context.
---
---@param target string|ArchetypeRef A directory name or Archetype("component-name")
---@param ctx Context Template context
---@param opts? RenderOpts
function archetype.render(target, ctx, opts) end

---Check if a switch is enabled (from CLI `--switch name` or config).
---@param name string Switch name
---@return boolean
function archetype.switch(name) end

---@class ExistingPolicy
---Strategy for handling files that already exist at the destination.

---@class Existing
---Policies for handling existing files during rendering.
---@field Overwrite ExistingPolicy Replace existing files
---@field Preserve ExistingPolicy Keep existing files unchanged
---@field Prompt ExistingPolicy Ask the user what to do
Existing = {}

---@class RenderOpts
---@field destination? string Subdirectory to render into (relative to current destination)
---@field if_exists? ExistingPolicy How to handle existing files (e.g., `Existing.Overwrite`)
---@field switches? string[] Switches to pass to child archetype
---@field use_defaults? string[] Keys to use defaults for in child archetype
---@field use_defaults_all? boolean Use defaults for all prompts in child archetype

--
-- Archetype (component reference)
--

---@class ArchetypeRef
---A reference to a registered child archetype component.
---Created by `Archetype("name")` where `name` matches a key in the
---`components` section of `archetype.yaml`.

---Create a reference to a registered archetype component.
---The component must be declared in the `components` section of `archetype.yaml`.
---@param name string Component name from archetype.yaml
---@return ArchetypeRef
function Archetype(name) end

--
-- template
--

---@class template
---Inline Jinja2 template rendering.
template = {}

---Render an inline Jinja2 template string using the context's variables.
---@param tmpl string Template string (e.g., "{{ project_name | title_case }}")
---@param ctx Context Template context
---@return string result The rendered string
function template.render(tmpl, ctx) end

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
