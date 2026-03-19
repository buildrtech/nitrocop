# Multi-line implicit `it` block (allow_single_line default)
block do
^ Style/ItBlockParameter: Avoid using `it` block parameter for multi-line blocks.
  do_something(it)
end
items.each do
^ Style/ItBlockParameter: Avoid using `it` block parameter for multi-line blocks.
  puts it
end
records.map do
^ Style/ItBlockParameter: Avoid using `it` block parameter for multi-line blocks.
  it.to_s
end
# Single numbered parameter _1 (should use `it`)
block { do_something(_1) }
                     ^^ Style/ItBlockParameter: Use `it` block parameter.
items.map { _1.to_s }
            ^^ Style/ItBlockParameter: Use `it` block parameter.
# Multiple _1 references in a block
block do
  foo(_1)
      ^^ Style/ItBlockParameter: Use `it` block parameter.
  bar(_1)
      ^^ Style/ItBlockParameter: Use `it` block parameter.
end
# Lambda with single numbered parameter _1
-> { do_something(_1) }
                  ^^ Style/ItBlockParameter: Use `it` block parameter.
# Multi-line lambda with implicit `it`
-> do
^ Style/ItBlockParameter: Avoid using `it` block parameter for multi-line blocks.
  it.to_s
end
