[1, 2, { one: 1, two: 2 }]
[{ one: 1 }, { two: 2 }]
[1, {}]
foo(one: 1, two: 2)
[1, 2, 3]
[1, 2].each { |x| puts x }

# Implicit arrays from method args - not flagged
method_call 1, 2, key: value

# All elements are hash nodes WITH braces - not flagged in braces mode
[{ one: 1 }, { two: 2 }, { three: 3 }]

# Second-to-last element is also a hash - not flagged in no_braces mode
[{ controller: "foo", action: "bar" }, { controller: "baz", action: "qux" }]

# Hash with kwsplat (double-splat) as first child - not flagged
[1, 2, { **opts }]
[1, 2, { **defaults, key: 'val' }]

# Unbraced hash with kwsplat as first child - not flagged (braces mode)
[1, **opts]
[1, **defaults, key: 'val']
[1, 2, **config]
