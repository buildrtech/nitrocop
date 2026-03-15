!!something
^^^^^^^^^^^ Style/DoubleNegation: Avoid the use of double negation (`!!`).

x = !!foo
    ^^^^^ Style/DoubleNegation: Avoid the use of double negation (`!!`).

!!nil
^^^^^ Style/DoubleNegation: Avoid the use of double negation (`!!`).

# !! not in the last position of a method body
def foo?
  foo
  !!test.something
  ^^^^^^^^^^^^^^^^^ Style/DoubleNegation: Avoid the use of double negation (`!!`).
  bar
end

# !! inside hash values in return position (always an offense)
def foo
  { bar: !!baz, quux: value }
         ^^^^^ Style/DoubleNegation: Avoid the use of double negation (`!!`).
end

# !! inside array values in return position (always an offense)
def foo
  [foo1, !!bar1, baz1]
         ^^^^^^ Style/DoubleNegation: Avoid the use of double negation (`!!`).
end

# !! inside multi-line hash in return position
def foo
  {
    bar: !!baz,
         ^^^^^ Style/DoubleNegation: Avoid the use of double negation (`!!`).
    quux: !!corge
          ^^^^^^^ Style/DoubleNegation: Avoid the use of double negation (`!!`).
  }
end

# !! inside multi-line array in return position
def foo
  [
    foo1,
    !!bar1,
    ^^^^^^ Style/DoubleNegation: Avoid the use of double negation (`!!`).
    !!baz1
    ^^^^^^ Style/DoubleNegation: Avoid the use of double negation (`!!`).
  ]
end

# !! not at return position inside unless
def foo?
  unless condition
    !!foo
    ^^^^^ Style/DoubleNegation: Avoid the use of double negation (`!!`).
    do_something
  end
end

# !! not at return position inside if/elsif/else
def foo?
  if condition
    !!foo
    ^^^^^ Style/DoubleNegation: Avoid the use of double negation (`!!`).
    do_something
  elsif other
    !!bar
    ^^^^^ Style/DoubleNegation: Avoid the use of double negation (`!!`).
    do_something
  else
    !!baz
    ^^^^^ Style/DoubleNegation: Avoid the use of double negation (`!!`).
    do_something
  end
end

# !! inside nested conditional where inner if ends before outer if/elsif
# RuboCop does NOT consider this return position because the inner conditional
# ends before the def body's last expression
def invite(username, invited_by, guardian)
  if condition_a
    if condition_b
      !!call_one(invited_by, guardian)
      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/DoubleNegation: Avoid the use of double negation (`!!`).
    else
      !!call_two(invited_by, guardian)
      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/DoubleNegation: Avoid the use of double negation (`!!`).
    end
  elsif condition_c
    !!generate_record(
    ^^^^^^^^^^^^^^^^^^ Style/DoubleNegation: Avoid the use of double negation (`!!`).
      invited_by,
      topic: self,
    )
  end
end

# !! in block body (not define_method) — not a return position
items.select do |item|
  !!item.active
  ^^^^^^^^^^^^^ Style/DoubleNegation: Avoid the use of double negation (`!!`).
end
