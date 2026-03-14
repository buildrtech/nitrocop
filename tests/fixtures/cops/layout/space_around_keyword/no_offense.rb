if x
  y
end
unless x
  y
end
while x
  y
end
until x
  y
end
case x
when 1
  y
end

# yield( is accepted — no space needed before paren
def foo
  yield(x)
end

# `when` as a method name, not a keyword
def when(condition, expression = nil)
  condition
end

# `.when(...)` as a method call on an object (e.g. Arel)
result = Arel::Nodes::Case.new.
  when(transition_table[:id].eq(most_recent_id)).then(db_true).
  else(not_most_recent_value)

# `&.when(...)` safe-navigation method call
obj&.when(condition)
