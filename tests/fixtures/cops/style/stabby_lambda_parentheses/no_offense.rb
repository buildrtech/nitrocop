f = ->(x) { x + 1 }

g = -> { puts "hello" }

h = ->(x, y) { x + y }

i = ->(a) do
  a * 2
end

j = -> { 42 }

# Numbered parameters (implicit, not explicit arguments)
k = -> { _1 + _2 }
l = -> { _1.to_s }

# it parameter (Ruby 3.4+, implicit)
m = -> { it + 1 }

# Empty parentheses (no actual arguments)
n = ->() { true }
