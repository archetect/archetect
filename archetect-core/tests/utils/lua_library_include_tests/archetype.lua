-- The library `test-lib` was eagerly staged at archetype load. Its
-- includes/ directory is now in the include resolver search list under
-- the catalog map key. The template at contents/output.txt does
-- `{% include "test-lib/banner.atl" %}` which resolves through the
-- staging dir.
local context = Context.new()
context:set("project_name", "smoke-test")

directory.render("contents", context)
