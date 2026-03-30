foo.to_set { |x| [x, x * 2] }

foo.to_set { |x, y| [x.to_s, y.to_i] }

items.to_set { |x| x.to_s }

foo.to_set(&:to_s)
