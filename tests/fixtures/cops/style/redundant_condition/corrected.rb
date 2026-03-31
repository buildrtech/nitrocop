x || y

a || b

foo || bar

x.nil? ? true : x

if a.empty?
  true
else
  a
end

# unless with condition == body (not else)
unless b
  b
else
  c
end

# no-else pattern: if cond; cond; end → "This condition is not needed."
do_something

# assignment branches: both branches assign to same variable
if foo
  @value = foo
else
  @value = 'bar'
end

# local variable assignment branches
if foo
  value = foo
else
  value = 'bar'
end

# method call branches with same receiver
if x
  X.find(x)
else
  X.find(y)
end

# ternary with method call condition
b.x || c

# ternary with function call condition
a = b(x) || c

# ternary predicate+true with number else
a.zero? ? true : 5

# constant path write assignment branches (FN fix)
if ENV['GIT_ADAPTER']
  Gollum::GIT_ADAPTER = ENV['GIT_ADAPTER']
else
  Gollum::GIT_ADAPTER = 'rugged'
end
