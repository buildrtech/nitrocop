[1, 2, 3].size
[1, 2, 3].length
[1, 2, 3].count { |x| x > 1 }
[1, 2, 3].count(1)
arr.size
# Variables/method calls — type unknown, don't flag
arr.count
items.where(active: true).count
User.count
# to_a/to_h with block or args — don't flag
(1..3).to_a.count { |x| x > 1 }
(1..3).to_a.count(1)
[[:a, 1]].to_h.count { |k, v| v > 0 }
# Array()/Hash() with block — don't flag
Array(1..5).count { |x| x > 3 }
Hash(key: :value).count { |k, v| v }
# .count with block via to_proc — don't flag
[1, 2, 3].count(&:nil?)
