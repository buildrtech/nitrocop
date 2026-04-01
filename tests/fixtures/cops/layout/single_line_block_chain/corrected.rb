example.select { |item| item.cond? }
  .join('-')

example.select { |item| item.cond? }
  &.join('-')

items.map { |x| x.to_s }
  .first

parent = Class.new { def foo(&block); block; end }
child = Class.new(parent) { def foo; super { break 1 }
  .call; end }
