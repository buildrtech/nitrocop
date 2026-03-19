a, b, _ = foo
      ^^ Style/TrailingUnderscoreVariable: Trailing underscore variable(s) in parallel assignment are unnecessary.

x, _, _ = bar
   ^^^^^^ Style/TrailingUnderscoreVariable: Trailing underscore variable(s) in parallel assignment are unnecessary.

first, second, _ = baz
               ^^ Style/TrailingUnderscoreVariable: Trailing underscore variable(s) in parallel assignment are unnecessary.

a, *_ = foo
   ^^^ Style/TrailingUnderscoreVariable: Trailing underscore variable(s) in parallel assignment are unnecessary.

_, b, _ = foo
      ^^ Style/TrailingUnderscoreVariable: Trailing underscore variable(s) in parallel assignment are unnecessary.

a, *_, _, _ = foo
   ^^^^^^^^^ Style/TrailingUnderscoreVariable: Trailing underscore variable(s) in parallel assignment are unnecessary.

a, (b, _) = foo
       ^^ Style/TrailingUnderscoreVariable: Trailing underscore variable(s) in parallel assignment are unnecessary.

_, = foo
^^ Style/TrailingUnderscoreVariable: Trailing underscore variable(s) in parallel assignment are unnecessary.

a, _ = foo
   ^^ Style/TrailingUnderscoreVariable: Trailing underscore variable(s) in parallel assignment are unnecessary.
