Hash[ary]
^^^^^^^^^ Style/HashConversion: Prefer `ary.to_h` to `Hash[ary]`.
Hash[a, b, c, d]
^^^^^^^^^^^^^^^^ Style/HashConversion: Prefer literal hash to `Hash[arg1, arg2, ...]`.
Hash[]
^^^^^^ Style/HashConversion: Prefer literal hash to `Hash[arg1, arg2, ...]`.
Hash[a: b, c: d]
^^^^^^^^^^^^^^^^ Style/HashConversion: Prefer literal hash to `Hash[key: value, ...]`.
result = Hash[items.map do |k, v|
         ^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashConversion: Prefer `ary.to_h` to `Hash[ary]`.
  [k, Hash[v.map { |x| [x, true] }]]
end]
Hash[records.map { |r| [r.id, Hash[r.attrs.map { |a| [a.name, a.value] }]] }]
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashConversion: Prefer `ary.to_h` to `Hash[ary]`.
