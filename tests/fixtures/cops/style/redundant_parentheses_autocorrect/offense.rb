x = (1)
    ^^^ Style/RedundantParentheses: Don't use parentheses around a literal.

x.y((z))
    ^^^ Style/RedundantParentheses: Don't use parentheses around a method argument.

"#{(foo)}"
   ^^^^^ Style/RedundantParentheses: Don't use parentheses around an interpolated expression.

(return(42))
^^^^^^^^^^^^ Style/RedundantParentheses: Don't use parentheses around a keyword.
