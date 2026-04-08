[1, 2, 3].reverse_each { |x| puts x }
[1, 2, 3].reverse_each do |x|
  puts x
end
arr.reverse_each { |item| process(item) }
