local ctx = Context.new()

-- Test basic set/get/has
ctx:set("name", "Alice")
log.info(tostring(ctx:get("name")))
log.info(tostring(ctx:has("name")))
log.info(tostring(ctx:has("missing")))

-- Test integer set/get
ctx:set("count", 42)
log.info(tostring(ctx:get("count")))

-- Test boolean set/get
ctx:set("enabled", true)
log.info(tostring(ctx:get("enabled")))

-- Test nil for missing key
log.info(tostring(ctx:get("nonexistent")))
