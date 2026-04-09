[1, 2, 3].filter_map { |x| x if x > 1 }
[1, 2, 3].filter_map { |x| x if x > 1 }
arr.filter_map { |item| transform(item) }
collection.filter_map(&:do_something)
collection.filter_map(&:do_something)
items.filter_map {|x|
  x.valid? ? x.name : nil
}
