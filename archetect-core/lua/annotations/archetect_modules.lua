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

---Create a new GitHub repository.
---Requires `GITHUB_TOKEN` environment variable.
---Returns `false` if the repository already exists.
---@param repo string Repository in "owner/repo" format
---@param opts? {visibility?: "public"|"private"|"internal"} Options (default: "private")
---@return boolean created Whether the repository was created
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
---programmatically. Method surface is documented in archetect-aml.

---@class ModelBuilder
---Programmatic builder for constructing models in Lua. See
---archetect-aml for available methods.

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
