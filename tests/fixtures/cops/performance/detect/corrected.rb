[1, 2, 3].detect { |x| x > 1 }
arr.detect { |item| item.valid? }
users.detect { |u| u.active? }
[1, 2, 3].reverse.detect { |x| x > 1 }
items.reverse.detect { |i| i.ready? }
[1, 2, 3].detect { |x| x > 1 }
arr.reverse.detect { |item| item.ok? }
[1, 2, 3].detect { |x| x > 1 }
arr.reverse.detect { |item| item.ok? }
[1, 2, 3].detect(&:even?)
[1, 2, 3].reverse.detect(&:even?)
[1, 2, 3].detect { |i| i % 2 == 0 }
[1, 2, 3].reverse.detect { |i| i % 2 == 0 }
[1, 2, 3].detect(&:even?)
[1, 2, 3].reverse.detect(&:even?)
[1, 2, 3].detect do |i|
  i % 2 == 0
end
lazy.detect(&:even?)
