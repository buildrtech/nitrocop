array.compact
array.compact!
array.reject { |e| e.empty? }
array.reject { |e| e.zero? }
hash.reject { |k, v| v.blank? }
x = [1, nil, 2].compact
# Method chain on block param - not equivalent to .compact
items.reject { |item| item.target_status.nil? }
entries.reject { |e| e.value.nil? }
# Multi-param blocks where nil check is NOT on the last parameter
data.reject { |key, _builds| key.nil? }
data.reject { |l, _| l.nil? }
data.reject { |_, val, _| val.nil? }
data.reject { |c, _| c.nil? }
# Multi-param select/filter where nil check is NOT on the last parameter
data.select { |key, _| !key.nil? }
data.filter { |first, _second| !first.nil? }
# select/filter with non-nil? checks should not be flagged
array.select { |e| !e.empty? }
hash.select { |k, v| !v.blank? }
array.filter { |e| !e.zero? }
