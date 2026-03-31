foo { |a, b| a + b }

baz { |item, val| item.to_s }

test do |a, b|
  a + b
end

lambda { |foo, bar| do_something(foo, bar) }
