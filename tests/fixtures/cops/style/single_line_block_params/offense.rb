foo.reduce { |a, b| a + b }
             ^^^^^^ Style/SingleLineBlockParams: Name `reduce` block params `|acc, elem|`.

foo.inject { |x, y| x + y }
             ^^^^^^ Style/SingleLineBlockParams: Name `inject` block params `|acc, elem|`.

bar.reduce { |sum, item| sum + item }
             ^^^^^^^^^^^ Style/SingleLineBlockParams: Name `reduce` block params `|acc, elem|`.

# Wrong names with leading underscores should still be flagged (with underscore prefix preserved)
File.foreach(name).reduce(0) { |_x, _y| 1 }
                               ^^^^^^^^ Style/SingleLineBlockParams: Name `reduce` block params `|_acc, _elem|`.

# Single wrong param should be flagged
test.reduce(x) { |b| foo(b) }
                 ^^^ Style/SingleLineBlockParams: Name `reduce` block params `|acc|`.
