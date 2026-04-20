-- The archetype-root entry in package.path lets this library's own shim
-- reach `lib/init.lua` via `require("lib")`. Consumers mounting this
-- library via `library: true` reach the same module as
-- `require("<map-key>")` through the staged-library wiring instead.
output.print(require("lib").hello())
