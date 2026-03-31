a = [1, 2, 3]

a = 'a'

a = 1

# Percent literal splat inside an array literal (method arg) — NOT exempt
foo(['a', 'b', 'c'])

bar(["x", "y", z])

baz([:a, :b, :c])

# when clause with percent literal splat — not a method argument, not exempt
case x
when 'foo', 'bar', 'baz'
  1
end

# Array.new splat in assignment
a = *Array.new(3) { 42 }

# Array.new splat with ::Array
a = *::Array.new(3) { 42 }

# Single-element array literal with Array.new in assignment context
a = [*Array.new(foo)]

# Array.new inside [] method call in assignment context — flagged
ns = NoteSet[*Array.new(n) { |i| notes[(d + (i * 2)) % size] }]

# Block form inside a multi-element array literal is still an offense
schema = [
  1,
  *Array.new(3) { |i| i },
  2,
]

# Nested call that is the direct RHS of an assignment is still an offense
result = obj.call(*Array.new(5) { [] })

# Direct assignment RHS method call from the corpus
link1 = fu_clean_components(*Array.new([n, 0].max, '..'), *srcdirs[i..-1])
