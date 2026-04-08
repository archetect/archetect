local context = Context.new()

-- Test catalog.render() with a path to a leaf entry
-- The "services/grpc" entry has pre-configured answers (service_name = "grpc-service")
catalog.render("services/grpc", context)

-- Log a message to verify we returned from catalog.render()
log.info("catalog render completed")
