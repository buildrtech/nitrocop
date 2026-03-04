[1, 2, 3].select { |x| x > 1 }.count
          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Count: Use `count` instead of `select...count`.
[1, 2, 3].reject { |x| x > 1 }.count
          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Count: Use `count` instead of `reject...count`.
arr.select { |item| item.valid? }.count
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Count: Use `count` instead of `select...count`.
[1, 2, 3].select { |e| e.even? }.size
          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Count: Use `count` instead of `select...size`.
[1, 2, 3].reject { |e| e.even? }.size
          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Count: Use `count` instead of `reject...size`.
[1, 2, 3].select { |e| e.even? }.length
          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Count: Use `count` instead of `select...length`.
{a: 1, b: 2}.reject { |e| e == :a }.length
             ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Count: Use `count` instead of `reject...length`.
arr.filter { |x| x > 2 }.count
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Count: Use `count` instead of `filter...count`.
arr.find_all { |x| x > 2 }.size
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Count: Use `count` instead of `find_all...size`.
arr.select(&:value).count
    ^^^^^^^^^^^^^^^^^^^^^ Performance/Count: Use `count` instead of `select...count`.
foo.reject(&:blank?).size
    ^^^^^^^^^^^^^^^^^^^^^ Performance/Count: Use `count` instead of `reject...size`.
arr.filter(&:even?).length
    ^^^^^^^^^^^^^^^^^^^^^^ Performance/Count: Use `count` instead of `filter...length`.
# multi-statement block body (RuboCop does flag these)
items.map do |r|
  x = r.to_s
  r.split(".").select { |s| s == "*" }.count
               ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Count: Use `count` instead of `select...count`.
end
# assignment inside single-statement block body (RuboCop flags these)
items.each { |r| x = r.values.select { |v| v > 0 }.count }
                              ^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Count: Use `count` instead of `select...count`.
items.map do |r|
  total = r.entries.reject { |e| e.blank? }.size
                    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Count: Use `count` instead of `reject...size`.
end
# multi-line chain — offense should be on the select/reject line
result = records
  .select(&:active?)
   ^^^^^^^^^^^^^^^^^^ Performance/Count: Use `count` instead of `select...count`.
  .count
data
  .reject { |d| d.nil? }
   ^^^^^^^^^^^^^^^^^^^^^^ Performance/Count: Use `count` instead of `reject...count`.
  .count
# find_all/select + size/length used in comparisons (still flagged)
dups = current.uniq.find_all { |u| current.find_all { |c| c == u }.length > 1 }
                                           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Count: Use `count` instead of `find_all...length`.
assert(revisions.all? {|rev| rev.find_all {|obj| obj.type == :XRef }.size == 1 })
                                 ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Count: Use `count` instead of `find_all...size`.
doc.methods(false).select { |e| e =~ /^find_by_/ }.size == doc.keys.size
                   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Count: Use `count` instead of `select...size`.
