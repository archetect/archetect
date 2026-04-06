if archetype.switch("feature_a") then
    log.info("feature_a_enabled")
end

if archetype.switch("feature_b") then
    log.info("feature_b_enabled")
end

if not archetype.switch("feature_c") then
    log.info("feature_c_disabled")
end
