def with_block(&block)
  items.each(&block)

  super { yield }

  items.each(1) { |x| yield x }
end
