# Multi-line implicit `it` block (allow_single_line default)
block do
  do_something(it)
end
items.each do
  puts it
end
records.map do
  it.to_s
end
# Single numbered parameter _1 (should use `it`)
block { do_something(it) }
items.map { it.to_s }
# Multiple _1 references in a block
block do
  foo(it)
  bar(it)
end
# Lambda with single numbered parameter _1
-> { do_something(it) }
# Multi-line lambda with implicit `it`
-> do
  it.to_s
end
# super with block containing _1 (ForwardingSuperNode)
super { [it.name, it] }
