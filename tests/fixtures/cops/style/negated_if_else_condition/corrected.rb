if !x
  do_something
else
  do_something_else
end

if not y
  a
else
  b
end

z ? do_something_else : do_something

# != operator is a negated condition
if x != y
  do_something
else
  do_something_else
end

# !~ operator is a negated condition
if x !~ y
  do_something
else
  do_something_else
end

# != in ternary
x == y ? do_something_else : do_something

# Parenthesized negated condition
if (!x)
  do_something
else
  do_something_else
end

# Parenthesized != condition
if (x != y)
  do_something
else
  do_something_else
end

# begin-end wrapped negated condition
if begin
  x != y
end
  do_something
else
  do_something_else
end

# Empty if-branch with negated condition
if !condition.nil?
else
  foo = 42
end
