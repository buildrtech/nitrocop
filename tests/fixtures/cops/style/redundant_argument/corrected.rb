array.join

exit

exit!

# Multiline receiver - offense at argument parens, not receiver start
[
  1,
  2,
  3
].join

# Chained multiline call
items
  .map(&:to_s)
  .join

str.split

str.chomp

str.to_i

arr.sum

# Block literal does not make the redundant argument non-redundant
arr.sum { |x| x * 2 }

result = ary.each.sum {|x| yielded << x; 2*x }

returned_object = "chunky bacon".split { |str| a << str.capitalize }

# do..end block also should not suppress detection
str.split do |s|
  puts s
end
