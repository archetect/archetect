if archetype.switch("test_runtime_error") then
    error("intentional error")
end

if archetype.switch("test_nil_index") then
    local x = nil
    local y = x.foo
end
