foo && bar
foo.bar && foo.baz
foo.bar || foo.baz
foo&.bar || foobar.baz
foo&.bar || foo.nil?
foo.nil? && foo&.bar
foo&.bar && foo.baz
foo&.bar && foo == 1
foo&.bar || foo == 1
foo&.bar && foo != 1
foo&.bar || foo != 1
foo&.bar && foo === 1
foo&.bar || foo === 1
foo == bar || foo == baz
return true if a.nil? || a&.whatever?
foo&.bar && foobar.baz && foo.qux
foobar.baz && foo&.bar && foo.qux
foobar.baz && foo&.bar && foo.qux && foo.foobar
foo&.bar || foo&.baz && foo.qux
foo&.bar && (foo.baz || foo.qux)
(foo&.bar && foo.baz) || foo.qux
foo&.bar && foo.baz = 1
foo&.bar && foo + 1
foo&.bar && foo - 1
foo&.bar && foo << 1
x.foo.bar && y.foo&.baz
