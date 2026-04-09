arr.sort_by(&:name)
array.sort_by(&:foo)
items.sort_by(&:size)
items.sort_by { |a| a[:foo] }
items.sort_by { |a| a['name'] }
array.sort_by!(&:foo)
array.min_by(&:foo)
array.max_by(&:foo)
array.minmax_by(&:foo)
items.sort_by! { |a| a[:key] }
items.min_by { |a| a[1] }
items
  .sort_by(&:name)
