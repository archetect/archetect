-- The consumer's own lib/ is on package.path implicitly. No
-- declaration in the manifest needed.
local greet = require("greet")
local nested = require("nested.util")

output.print(greet.hello("world"))
output.print(nested.shout(greet.hello("nested")))
