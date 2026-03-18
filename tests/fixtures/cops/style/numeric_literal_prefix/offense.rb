num = 0X12AB
      ^^^^^^ Style/NumericLiteralPrefix: Use 0x for hexadecimal literals.

num = 0B10101
      ^^^^^^^ Style/NumericLiteralPrefix: Use 0b for binary literals.

num = 0D1234
      ^^^^^^ Style/NumericLiteralPrefix: Do not use prefixes for decimal literals.

num = 0d1234
      ^^^^^^ Style/NumericLiteralPrefix: Do not use prefixes for decimal literals.

# Plain octal without 'o' prefix
num = 01234
      ^^^^^ Style/NumericLiteralPrefix: Use 0o for octal literals.

# Uppercase octal prefix
num = 0O1234
      ^^^^^^ Style/NumericLiteralPrefix: Use 0o for octal literals.

# Negative plain octal (RuboCop flags the integer part after the minus)
num = -01234
       ^^^^^ Style/NumericLiteralPrefix: Use 0o for octal literals.

# Negative uppercase octal
num = -0O10
       ^^^^ Style/NumericLiteralPrefix: Use 0o for octal literals.
