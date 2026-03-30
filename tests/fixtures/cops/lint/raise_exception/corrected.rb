raise StandardError, "message"
raise StandardError.new("message")
fail StandardError
raise ::StandardError, "oops"
raise ::StandardError.new("oops")
