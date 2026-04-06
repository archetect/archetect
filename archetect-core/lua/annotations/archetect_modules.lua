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

---Stage files matching a pattern.
---@param pattern string Glob pattern (e.g., "*.rs", "src/")
function GitRepo:add(pattern) end

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
