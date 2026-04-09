ancestors.flat_map(&:instance_methods)
items.flat_map(&:children)
[1, 2, 3].flat_map { |x| [x, x] }
items.flat_map { |x| [x, x] }
ancestors.flat_map(&:instance_methods)
ancestors.reject { |klass| klass == self }
  .flat_map(&:instance_methods)
