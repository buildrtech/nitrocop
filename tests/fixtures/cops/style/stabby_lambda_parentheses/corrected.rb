f = ->(x) { x + 1 }

g = ->(x, y) { x + y }

h = ->(a) do
  a * 2
end

f = ->(a=a()) { a }
