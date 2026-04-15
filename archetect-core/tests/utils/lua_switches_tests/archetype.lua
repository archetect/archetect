if archetect.switches.is_enabled("feature_a") then
    log.info("feature_a_enabled")
end

if archetect.switches.is_enabled("feature_b") then
    log.info("feature_b_enabled")
end

if not archetect.switches.is_enabled("feature_c") then
    log.info("feature_c_disabled")
end
