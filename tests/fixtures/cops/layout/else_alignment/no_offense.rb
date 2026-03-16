if foo
  bar
else
  baz
end

if foo
  bar
elsif qux
  baz
else
  quux
end

x = true ? 1 : 2

# Assignment context (keyword style): else aligns with if
links = if enabled?
          bar
        else
          baz
        end

# else/elsif correctly aligned with `if` keyword
x = if foo
      bar
    elsif qux
      baz
    else
      quux
    end

# else aligned with `if` keyword
y = if condition
      value_a
    else
      value_b
    end

# Single-line if/then/else/end — no alignment check needed
if val then puts "true" else puts "false" end
x = if cond then 'a' else 'b' end
result = defined?(if x then 'x' else '' end)