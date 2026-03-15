def m(&block)
  items.something(&block)
end

def n
  items.something { |i, j| yield j, i }
end

def o
  items.something { |i| do_something(i) }
end

def p
  yield
end

# Block with yield outside a method definition - not an offense
render("partial") do
  yield
end

items.each do |x|
  yield x
end

collection.map { |item| yield item }
