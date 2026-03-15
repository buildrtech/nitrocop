a { b }.c

foo = a do
  b
end
foo.c

items.each { |i| i.process }.size

# Chained block (both do..end) - the outer method also has a block
a do
  b
end.c do
  d
end

# Single-line chained block
a do b end.c do d end

# Brace block chained
items.map { |x| x.to_s }.join(", ")

# Lambda with brace block
-> { x }.call

# No chained method
a do
  b
end
