if condition_a
  action_a
elsif condition_b
  action_b
else
  action_c
end
if a
  1
else
  2
end
if a
  action_a
end
# ternary with if in else branch should not be flagged (RuboCop skips ternaries)
x = a ? b : if c then d else e end
result = condition ?
  value_a :
  if other_condition
    value_b
  else
    value_c
  end
# unless inside else should not be flagged
if a
  blah
else
  unless b
    foo
  end
end
# ternary inside else should not be flagged
if a
  blah
else
  c ? d : e
end
# semicolon-style if inside else should not be flagged (RuboCop crashes on autocorrect)
if a
  blah
else
  if b; foo; end
end
if a
  blah
else
  if b; foo; else; bar; end
end
