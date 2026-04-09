[1, 2, 3].select { |x| x > 1 }.map { |x| x * 2 }
[1, 2, 3].filter { |x| x > 1 }.map { |x| x * 2 }
arr.select { |item| item.valid? }.map { |item| item.name }
ary.filter_map { |o| o.to_i if o.present? }
ary.filter_map { |o| o.to_i if o.present? }
ary.do_something.filter_map { |o| o.to_i if o.present? }.max
filter_map { |o| o.to_i if o.present? }
# select without a block (returns Enumerator) chained with map
select.map { |e| e.to_s }
items.select.map { |e| e.name }
# select inside block body, chained with .map on block result
items.flat_map { |g| g.users.select(&:active?) }.map(&:name)
