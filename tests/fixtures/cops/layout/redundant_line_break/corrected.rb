my_method(1) \ [:a]

foo && \
  bar

foo || \
  bar

my_method(1, 2, "x")

foo(' .x').bar.baz

a = m(1 + 2 + 3)

b = m(4 + 5 + 6)

raise ArgumentError, "can't inherit configuration from the rubocop gem"

foo(x, y, z).bar.baz

x = [ 1, 2, 3 ]

y = { a: 1, b: 2 }

foo( bar(1, 2) )

@count += items.size

@@total += items.size

$counter += items.size

@cache ||= compute_value

@flag &&= check_flag

# Multiline regex — RuboCop's safe_to_split? does not check :regexp,
# so assignments containing multiline regexps are still flaggable.
pattern = / \A (?<key>.+) \z /x

# Multiline %w array — RuboCop's safe_to_split? does not check arrays.
names = %w[ alpha beta gamma ]
