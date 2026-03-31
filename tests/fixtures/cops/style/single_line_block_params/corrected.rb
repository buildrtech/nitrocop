foo.reduce { |acc, elem| acc + elem }

foo.inject { |acc, elem| acc + elem }

bar.reduce { |acc, elem| acc + elem }

# Wrong names with leading underscores should still be flagged (with underscore prefix preserved)
File.foreach(name).reduce(0) { |_acc, _elem| 1 }

# Single wrong param should be flagged
test.reduce(x) { |acc| foo(acc) }
