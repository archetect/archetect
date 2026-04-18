-- Simplest archetype for gRPC roundtrip tests:
-- one text prompt, one log line, one file write. Exercises PromptForText +
-- LogInfo + WriteDirectory + WriteFile + CompleteSuccess on the server side,
-- matched by String response + Ack acknowledgements on the client side.
local context = Context.new()

context:prompt_text("Name:", "name")

log.info("rendering for " .. context:get("name"))

directory.render("contents", context)
