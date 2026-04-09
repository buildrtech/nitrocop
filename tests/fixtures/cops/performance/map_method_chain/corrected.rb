x.map { |e| e.foo.bar }
arr.map { |e| e.to_s.upcase }
items.map { |e| e.name.downcase }
# Triple chain should fire only once
items.map(&:a).map { |e| e.b.c }
# Without receiver
map { |e| e.foo.bar }
# Safe navigation on first call
items&.map(&:foo).map(&:bar)
