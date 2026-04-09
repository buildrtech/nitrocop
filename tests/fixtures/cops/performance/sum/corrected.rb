[1, 2, 3].sum
[1, 2, 3].sum
[1, 2, 3].sum
[1, 2, 3].sum
items.sum
body.map(&:bytesize).sum
items.inject(0) { |sum, n| sum + n }
items.inject(0) { |sum, n| n + sum }
items.reduce(0) { |acc, val| acc + val }
items.inject { |sum, x| sum + x }
items.reduce { |sum, x| sum + x }
items.inject(10) { |sum, n| sum + n }
values.inject(0.0) { |sum, v| sum + v }
items.sum(init)
items.sum(init)
items.sum(10)
items.inject(&:+)
items.reduce(&:+)
items.inject(0, &:+)
items.reduce(0, &:+)
items.map { |x| x.value }.sum
items.collect { |x| x ** 2 }.sum
items.map(&:count).sum
items.map { |x| x.value }.sum(10)
sum
sum
inject { |sum, x| sum + x }
reduce(0) { |acc, e| acc + e }
inject(0, :+) do |count|
  percentage = count.to_f / 10
end
