a do
  b
end.c
^ Style/MethodCalledOnDoEndBlock: Avoid chaining a method call on a do...end block.

foo do |x|
  x
end.count
^ Style/MethodCalledOnDoEndBlock: Avoid chaining a method call on a do...end block.

items.each do |i|
  i.process
end.size
^ Style/MethodCalledOnDoEndBlock: Avoid chaining a method call on a do...end block.

# Single-line do..end with chained call
a do b end.c
       ^ Style/MethodCalledOnDoEndBlock: Avoid chaining a method call on a do...end block.

# Lambda do..end with chained call
-> do
  x
end.call
^ Style/MethodCalledOnDoEndBlock: Avoid chaining a method call on a do...end block.

# Block argument on the chained method should still fire
a do
  b
end.c(&blk)
^ Style/MethodCalledOnDoEndBlock: Avoid chaining a method call on a do...end block.

# Safe navigation operator
a do
  b
end&.c
^ Style/MethodCalledOnDoEndBlock: Avoid chaining a method call on a do...end block.

# Chained method on a different line from end
# nitrocop-expect: 34:0 Style/MethodCalledOnDoEndBlock: Avoid chaining a method call on a do...end block.
items.map do |x|
  x.to_s
end
  .join(", ")
