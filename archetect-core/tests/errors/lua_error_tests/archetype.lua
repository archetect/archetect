if archetect.switches.is_enabled("test_runtime_error") then
    error("intentional error")
end

if archetect.switches.is_enabled("test_nil_index") then
    local x = nil
    local y = x.foo
end
