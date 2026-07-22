---@meta archetect_modules
--- Archetect v3 — Type annotations for require-based modules
--- These modules are loaded with: local mod = require("archetect.module_name")

--
-- archetect.shell
--

---@class archetect.shell
local shell = {}

---Run a command. Raises an error if the command fails.
---@param program string The program to run
---@param args? string[] Command arguments
---@param opts? {cwd?: string, env?: table<string,string>} Options
function shell.run(program, args, opts) end

---Run a command and capture stdout. Raises an error if the command fails.
---@param program string The program to run
---@param args? string[] Command arguments
---@param opts? {cwd?: string, env?: table<string,string>} Options
---@return string stdout The captured output (trimmed)
function shell.capture(program, args, opts) end

--
-- archetect.git
--

---@class archetect.git
local git = {}

---Initialize a git repository.
---@param path? string Path relative to destination. Defaults to destination root.
---@param opts? {branch?: string} Options (e.g., initial branch name)
---@return GitRepo repo A handle to the initialized repository
function git.init(path, opts) end

---@class GitRepo
---A handle to a git repository, returned by `git.init()`.
local GitRepo = {}

---Stage files matching one or more patterns. Accepts either a single
---string or a Lua array of strings — both fan into a single
---`git add` invocation.
---@param patterns string|string[] Glob pattern or list of patterns
function GitRepo:add(patterns) end

---Stage all changes.
function GitRepo:add_all() end

---Commit staged changes.
---@param message string Commit message
function GitRepo:commit(message) end

---Create a new branch.
---@param name string Branch name
function GitRepo:branch(name) end

---Check out a branch.
---@param name string Branch name
function GitRepo:checkout(name) end

---Add a remote.
---@param name string Remote name (e.g., "origin")
---@param url string Remote URL
function GitRepo:remote_add(name, url) end

---Push to a remote.
---@param remote string Remote name
---@param branch string Branch name
function GitRepo:push(remote, branch) end

--
-- archetect.github
--

---@class archetect.github
local github = {}

---Check if a GitHub repository exists.
---Requires `GITHUB_TOKEN` environment variable.
---@param repo string Repository in "owner/repo" format
---@return boolean exists
function github.repo_exists(repo) end

---Create a new GitHub repository, or report on an existing one.
---Requires `GITHUB_TOKEN` environment variable.
---Returns a table describing the outcome. `created` is true iff this call
---newly created the repo. `empty` is true iff the repo has no commits —
---which callers use to decide whether it is safe to push without clobbering.
---Raises on auth/network failure, malformed slug, or GitHub-side rejection.
---@param repo string Repository in "owner/repo" format
---@param opts? {visibility?: "public"|"private"|"internal"} Options (default: "private")
---@return {created: boolean, empty: boolean}
function github.create_repo(repo, opts) end

--
-- archetect.archive
--

---@class archetect.archive
local archive = {}

---Create a ZIP archive from a source directory.
---Paths are relative to the render destination.
---@param source string Source directory path
---@param destination string Output archive path
function archive.zip(source, destination) end

---Create a gzipped tar archive from a source directory.
---Paths are relative to the render destination.
---@param source string Source directory path
---@param destination string Output archive path
function archive.tar_gz(source, destination) end

---Create a tar archive (uncompressed) from a source directory.
---Paths are relative to the render destination.
---@param source string Source directory path
---@param destination string Output archive path
function archive.tar(source, destination) end

--
-- archetect.model — AML (Archetect Modeling Language) model loading
--
-- Required as: `local model = require("archetect.model")`
-- AML is a YAML-shaped DSL for describing a domain (entities, fields,
-- relationships, etc.) that an archetype can iterate over to generate
-- code. See `archetect-aml/` for the language reference.

---@class archetect.model
local model = {}

---Load and parse a model from a YAML file.
---@param path string Path to an AML / YAML file
---@return Model
function model.load(path) end

---Parse a model from a YAML string.
---@param yaml string AML / YAML source
---@return Model
function model.parse(yaml) end

---Construct a fresh model programmatically via builder methods.
---@return ModelBuilder
function model.builder() end

---Load a model from the current Context's answers. Looks for
---`model_path` (file) then `model_yaml` (inline string); errors if
---neither is set.
---@param context Context
---@return Model
function model.from_context(context) end

---@class Model
---A loaded AML model. Pass to templates via context, or iterate
---programmatically via the query methods below. Entities come back
---expanded: fields carry pre-computed case variants, so templates can
---case-address them directly.
Model = {}

---Look up one entity by name, expanded (fields with case variants).
---@param name string Entity name
---@return table|nil entity The expanded entity, or nil if unknown
function Model:entity(name) end

---Look up one service boundary by name.
---@param name string Boundary name
---@return table|nil boundary The boundary, or nil if unknown
function Model:boundary(name) end

---Every service boundary in the model, in declaration order.
---@return table[] boundaries
function Model:all_boundaries() end

---Boundaries whose `type` matches (e.g. "grpc").
---@param btype string Boundary type
---@return table[] boundaries
function Model:boundaries_of_type(btype) end

---The expanded entities a boundary owns.
---@param boundary_name string Boundary name
---@return table[] entities
function Model:entities_for(boundary_name) end

---Interfaces this boundary consumes from others (client side).
---@param name string Boundary name
---@return table[] interfaces
function Model:outbound_interfaces(name) end

---Interfaces other boundaries consume from this one (server side).
---@param name string Boundary name
---@return table[] interfaces
function Model:inbound_interfaces(name) end

---Names of the boundaries this boundary depends on — the service DAG
---edge list for one node.
---@param name string Boundary name
---@return string[] dependencies
function Model:dependencies(name) end

---Cross-boundary entity references from this boundary — the entities it
---reads but does not own (each implies a client stub).
---@param name string Boundary name
---@return table[] references
function Model:remote_references(name) end

---Everything one boundary needs in a single shape: its entities,
---interfaces, dependencies, and remote references. Errors on an unknown
---boundary name.
---@param name string Boundary name
---@return table slice
function Model:slice(name) end

---The organization + solution pair with case variants pre-computed.
---@return table cases
function Model:org_solution() end

---The model's organization name.
---@return string
function Model:organization() end

---The model's solution name.
---@return string
function Model:solution() end

---@class ModelBuilder
---Programmatic builder for constructing models in Lua. Set identity,
---add entities/fields/relations/boundaries/interfaces, then `build()`.
ModelBuilder = {}

---Set the organization name.
---@param org string
function ModelBuilder:set_organization(org) end

---Set the solution name.
---@param sol string
function ModelBuilder:set_solution(sol) end

---Set the model description.
---@param desc string
function ModelBuilder:set_description(desc) end

---Add an entity by name.
---@param name string
function ModelBuilder:add_entity(name) end

---Add a simple typed field to an entity.
---@param entity string Entity name
---@param field_name string
---@param field_type string Type name (e.g. "string", "decimal")
function ModelBuilder:add_field(entity, field_name, field_type) end

---Add a relation field to an entity.
---@param entity string Entity name
---@param field_name string
---@param target string Target entity name
---@param relation string Relation kind (e.g. "many_to_one")
---@param required boolean
function ModelBuilder:add_relation(entity, field_name, target, relation, required) end

---Add a service boundary owning the listed entities.
---@param name string Boundary name
---@param btype string Boundary type (e.g. "grpc")
---@param owns string[] Entity names this boundary owns
function ModelBuilder:add_boundary(name, btype, owns) end

---Add an interface between two boundaries.
---@param from string Consuming boundary
---@param to string Providing boundary
---@param style string Interface style
function ModelBuilder:add_interface(from, to, style) end

---Finalize and return the Model. Resets the builder.
---@return Model
function ModelBuilder:build() end

--
-- archetect.model.interactive — Lua-implemented interactive builder
--
-- Required as: `local interactive = require("archetect.model.interactive")`
-- Wraps the model builder with prompt-driven entry — useful for
-- archetypes that want the user to author the model interactively
-- rather than supply it via answer file.

---@class archetect.model.interactive
local interactive = {}

---Run the interactive model builder against the given context and
---return the resulting Model.
---@param context Context
---@return Model
function interactive.build(context) end
