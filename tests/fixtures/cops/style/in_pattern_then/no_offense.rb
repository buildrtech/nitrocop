case a
in b then c
end
case e
in f
  g; h
end
case condition
in pattern
end
case x
in b then c; d
end
# "in " at the start of a line in non-pattern-matching context
x = "in the beginning"
y = "something in mind; more"
# Semicolon with then keyword — then takes precedence
case value
in 2; then puts "two"
in 4; then puts "four"
end
# Multiline in pattern — body on next line
case [0, 1, 2, 3]
in 0, 1,;
  true
end
# Multiline in with splat
case [0, 1]
in *;
  true
end
# Multiline in with double splat
case {a: 1}
in **;
  true
end
