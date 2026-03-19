# Single-line implicit `it` block is OK with allow_single_line
block { do_something(it) }
items.map { it.to_s }
records.select { it > 0 }
# Named block parameters are fine
block { |arg| do_something(arg) }
items.each { |item| puts item }
# Named `it` parameter is fine (not the same as implicit `it`)
foo.each { |it| puts it }
# Multiple numbered parameters are fine (not single _1)
block { do_something(_1, _2) }
# Single _2 is fine (not _1)
block { do_something(_2) }
# Multi-line method chain with single-line it block is fine
collection.each
          .foo { puts it }
# Single-line lambda with implicit `it` is OK
-> { do_something(it) }
# Lambda with multiple numbered params is fine
-> { _1 + _2 }
# Lambda with single _2 is fine
-> { _2 }
