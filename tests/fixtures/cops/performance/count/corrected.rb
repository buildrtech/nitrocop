[1, 2, 3].count { |x| x > 1 }
[1, 2, 3].count { |x| !(x > 1) }
arr.count { |item| item.valid? }
[1, 2, 3].count { |e| e.even? }
[1, 2, 3].count { |e| !(e.even?) }
[1, 2, 3].count { |e| e.even? }
{a: 1, b: 2}.count { |e| !(e == :a) }
arr.count { |x| x > 2 }
arr.count { |x| x > 2 }
arr.count(&:value)
foo.count { |element| !element.blank? }
arr.count(&:even?)
# multi-statement block body (RuboCop does flag these)
items.map do |r|
  x = r.to_s
  r.split(".").count { |s| s == "*" }
end
# assignment inside single-statement block body (RuboCop flags these)
items.each { |r| x = r.values.count { |v| v > 0 } }
items.map do |r|
  total = r.entries.count { |e| !(e.blank?) }
end
# multi-line chain — offense should be on the select/reject line
result = records.count(&:active?)
data.count { |d| !(d.nil?) }
# find_all/select + size/length used in comparisons (still flagged)
dups = current.uniq.find_all { |u| current.count { |c| c == u } > 1 }
assert(revisions.all? {|rev| rev.count {|obj| obj.type == :XRef } == 1 })
doc.methods(false).count { |e| e =~ /^find_by_/ } == doc.keys.size
