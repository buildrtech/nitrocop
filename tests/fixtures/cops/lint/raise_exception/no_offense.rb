raise StandardError, "message"
raise RuntimeError.new("message")
raise "message"
fail ArgumentError, "bad"
raise TypeError
raise ::StandardError, "qualified"
raise ::Foreman::Exception.new("msg")
raise Foreman::Exception.new("msg")
raise MyApp::Exception
raise Foo::Bar::Exception, "namespaced"
# raise Exception.new as first arg with extra args to raise — RuboCop doesn't match this pattern
raise Exception.new, "message"
raise Exception.new, @valid ? "ok" : "bad"
raise(Exception.new, "with parens")
