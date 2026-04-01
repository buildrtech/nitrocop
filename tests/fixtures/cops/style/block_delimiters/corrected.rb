items.each { |x| puts x }

items.map do
  |x| x * 2
end

[1, 2].each { |i| i + 1 }

items.map do
  items.select {
    true
  }
end

# Chained blocks: only the outermost (last in chain) is flagged
items.select {
  x.valid?
}.reject {
  x.empty?
}.each do
  puts x
end

# super with args and multi-line braces should be flagged
super(arg) do
  do_something
end

# forwarding super (no args) with multi-line braces
super do
  yield if block_given?
  process
end

expect do
  subject.log("a")
  subject.debug("b")
end.to output("a\nb\n").to_stdout
