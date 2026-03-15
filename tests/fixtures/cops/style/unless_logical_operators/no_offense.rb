unless a && b
  do_something
end

unless x || y
  do_something
end

unless condition
  do_something
end

# Properly grouped mixed operators — parentheses isolate subexpressions
unless (a || b) && c
  do_something
end

unless (a && b) || c
  do_something
end

unless (a || b) && (c || d)
  do_something
end

x = 1
y = 2
