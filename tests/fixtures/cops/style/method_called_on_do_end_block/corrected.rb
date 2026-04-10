a {
  b
}.c

foo { |x|
  x
}.count

items.each { |i|
  i.process
}.size

# Single-line do..end with chained call
a { b }.c

# Lambda do..end with chained call
-> {
  x
}.call

# Block argument on the chained method should still fire
a {
  b
}.c(&blk)

# Safe navigation operator
a {
  b
}&.c

# Chained method on a different line from end
items.map { |x|
  x.to_s
}
  .join(", ")
