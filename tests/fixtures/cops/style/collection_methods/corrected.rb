[1, 2, 3].map { |e| e + 1 }

[1, 2, 3].reduce { |a, e| a + e }

[1, 2, 3].find { |e| e > 1 }

# Block pass (&:sym) should also be flagged
[1, 2, 3].map(&:to_s)

[1, 2, 3].find(&:odd?)

# Symbol arg for MethodsAcceptingSymbol methods
[1, 2, 3].reduce(:+)

[1, 2, 3].reduce(0, :+)

# member? with a block should be flagged
[1, 2, 3].include? { |e| e > 1 }
