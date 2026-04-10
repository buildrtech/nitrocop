Hash.new { |h, k| h[k] = [] }
Hash.new { |h, k| h[k] = {} }
Hash.new(Array.new)
Hash.new(unknown: true)
Hash.new(foo: 'bar', baz: 42)
Hash.new(Hash.new)
Hash.new(unknown: true) { 0 }
Hash.new([]) { |h, k| h[k] = [] }
