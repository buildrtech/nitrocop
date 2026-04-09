str.casecmp("other").zero?
str.casecmp("OTHER").zero?
str.casecmp('string').zero?
str.casecmp('string').zero?

# != operator
!str.casecmp('string').zero?
!str.casecmp('string').zero?

# Reversed operand order (string literal on LHS)
system.casecmp("english").zero?
key.casecmp('CONTENT-TYPE').zero?

# eql? method
str.casecmp("foo").zero?
str.casecmp('BAR').zero?

# downcase == downcase
str.casecmp(str.downcase).zero?
str.casecmp(other.upcase).zero?

# Parenthesized string
str.casecmp("foo").zero?

# Explicit empty parentheses on downcase/upcase (same as without parens)
str.casecmp("other").zero?
str.casecmp("OTHER").zero?
!str.casecmp("other").zero?
!("header" || "").casecmp(origin.downcase()).zero?
