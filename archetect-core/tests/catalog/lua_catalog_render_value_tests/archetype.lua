-- The parent collects some local state, then asks the org-prompts
-- component to contribute. The component's mutations appear ONLY in
-- the returned context, not in the parent's `parent_local`.
local parent_local = Context.new()
parent_local:set("set-by-parent", "yes")

-- Sandbox: store the child's result in a separate variable. The parent's
-- local context is not affected.
local from_child = catalog.render("org-prompts", parent_local)

if from_child:has("set-by-component") then
    output.print("child returned: " .. (from_child:get("set-by-component") or "nil"))
else
    output.print("child returned no key")
end

if parent_local:has("set-by-component") then
    output.print("parent leaked")
else
    output.print("parent isolated")
end

-- Replace pattern: assign back to a fresh variable. The new variable
-- has the child's full state. The original parent_local is still
-- unchanged.
local merged = catalog.render("org-prompts", parent_local)
if merged:has("set-by-parent") and merged:has("set-by-component") then
    output.print("merged has both")
else
    output.print("merged missing keys")
end
