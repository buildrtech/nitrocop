some_method(
  a,
  b
)

some_method(
  a,
  b
)

other_method(
  x,
  y
)

# Grouped expression with hanging )
w = x * (
  y + z
)

# Nested call: first arg on next line, `)` under-indented
class Foo
  def bar
    method_call(
      arg1,
      arg2
    )
  end
end

# Scenario 2 with args on same line: `)` should align with `(`
some_method(a
           )

# Def with first param on same line: `)` should align with `(`
def some_method(a
               )
end

# No-args call with hanging paren: `)` misaligned
some_method(
)

# Def with no params: `)` misaligned
def some_method(
)
end

# Scenario 2: aligned args, `)` not aligned with `(`
some_method(a,
            b,
            c
           )

# Scenario 2: unaligned args, `)` misindented
some_method(a,
  x: 1,
  y: 2
)

# Indented no-args call: `)` misaligned
class Foo
  def bar
    some_method(
    )
  end
end

# Method assignment: no args, `)` misaligned
foo = some_method(
)
