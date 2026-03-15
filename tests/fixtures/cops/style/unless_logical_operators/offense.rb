unless a && b || c
^^^^^^ Style/UnlessLogicalOperators: Do not use mixed logical operators in `unless` conditions.
  do_something
end

unless x || y && z
^^^^^^ Style/UnlessLogicalOperators: Do not use mixed logical operators in `unless` conditions.
  do_something
end

unless foo && bar || baz
^^^^^^ Style/UnlessLogicalOperators: Do not use mixed logical operators in `unless` conditions.
  do_something
end

# Mixed precedence: && with and
unless a && b and c
^^^^^^ Style/UnlessLogicalOperators: Do not use mixed logical operators in `unless` conditions.
  do_something
end

# Mixed precedence: || with or
unless a || b or c
^^^^^^ Style/UnlessLogicalOperators: Do not use mixed logical operators in `unless` conditions.
  do_something
end
