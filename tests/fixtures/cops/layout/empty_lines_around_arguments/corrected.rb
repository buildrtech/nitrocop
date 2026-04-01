# Empty line between args
foo(
  bar,
  baz
)

# Empty line between args
something(
  first,
  second
)

# Empty line between first and rest
method_call(
  a,
  b,
  c
)

# Empty line before first arg
do_something(
  bar
)

# Empty line after last arg before closing paren
bar(
  [baz, qux]
)

# Args start on definition line with empty line
foo(biz,
    baz: 0)

# Multiple empty lines (3 offenses)
multi(
  baz,
  qux,
  biz,
)

# Multiple blank lines in one gap still report only the last blank line
double_gap(
  foo,
  bar
)

# Whitespace-only blank line before closing paren still counts
space_gap(
  value
)

# Whitespace-only blank line between args still counts
space_between(
  alpha,
  beta
)
