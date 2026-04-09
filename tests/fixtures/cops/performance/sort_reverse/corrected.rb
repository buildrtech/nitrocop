[3, 1, 2].sort.reverse
arr.sort.reverse
items.sort.reverse
# multiline chained receiver — offense should be at .sort, not receiver start
items.select { |x| x > 0 }.sort.reverse
