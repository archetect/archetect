-- Intentionally raises a runtime error so the gRPC flow can verify that
-- script aborts surface as a LogError on the stream (current server
-- behavior) or a CompleteError in future revisions.
error("intentional test failure")
