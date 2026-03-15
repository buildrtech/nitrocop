x&.foo&.bar
x&.foo
x.foo.bar.baz
x&.foo.bar
x = 1
y = x&.to_s

# Block breaks safe navigation chain — each segment counted separately
parsed_lockfile&.specs&.find { |s| s.name == dependency_name }&.source
sources = requirements&.map { |r| r.fetch(:source) }&.uniq&.compact
items&.select { |i| i.valid? }&.map { |i| i.name }&.join(", ")
result&.data&.each { |d| process(d) }&.count
