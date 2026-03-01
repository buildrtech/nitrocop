arr.sort_by(&:name)
arr.sort
arr.sort { |a, b| a <=> b }
array.sort_by { |a| a.baz }
array.sort { |a, b| a.foo <=> b.bar }
# safe navigation (&.) should not be flagged
items&.sort { |a, b| a.name <=> b.name }
# method calls with arguments should not be flagged
items.sort { |a, b| a.fetch(:key) <=> b.fetch(:key) }
# reversed parameter order should not be flagged
array.sort { |a, b| b.foo <=> a.foo }
array.min { |a, b| b.foo <=> a.foo }
array.max { |a, b| b.foo <=> a.foo }
# safe navigation on other methods should not be flagged either
items&.min { |a, b| a.name <=> b.name }
items&.max { |a, b| a.name <=> b.name }
# sort/max/min with arguments should not be flagged (not simple Enumerable calls)
vector.sort(ascending: false) { |a, b| a.to_s <=> b.to_s }
dv.max(2) { |a, b| a.size <=> b.size }
dv.min(2) { |a, b| a.size <=> b.size }
