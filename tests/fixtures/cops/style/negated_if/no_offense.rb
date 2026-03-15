unless x
  do_something
end

if !x
  do_something
else
  do_other
end

if x
  do_something
end

do_something unless x

if x && y
  do_something
end

# Double negation (!! is NOT a single negation)
if !!condition
  do_something
end

return if !!ENV["testing"]

something if !!value

# Negation as only part of a compound condition
if !condition && another_condition
  do_something
end

some_method if not condition or another_condition

# if/else with negated condition is accepted
if not a_condition
  some_method
elsif other_condition
  something_else
end

# Pattern match guard — `unless` is invalid syntax in guard clauses
case node
in :property if !allowed?(value)
  nil
end

case x
in :foo if !bar
  nil
end

# Safe-navigation chain ending in &.! — rewriting to unless is problematic
if @options&.empty?&.!
  process_options
end
