one_plus_one = 1 \
  + 1

two_plus_two = 2 \
  + 2

three = 3 \
  + 0

# Backslash inside heredoc should not be flagged
x = <<~SQL
  SELECT * FROM users \
  WHERE id = 1
SQL

y = <<~SHELL
  echo hello \
  world
SHELL

z = <<~RUBY
  foo(bar, \
      baz)
RUBY

# String concatenation with exactly one space before backslash (correct)
raise TypeError, "Argument must be a Binding, not " \
    "a #{scope.class.name}"

msg = "Expected result " \
    "but found something else"

debugMsg(2, "Starting at position %d: prefix = %s, " \
         "delimiter = %s, quoted = %s",
         pos, prefix, delim, quoted)

# Single-quoted string concatenation with one space before backslash
result = 'hello ' \
    'world'

# Backslash inside multiline string should not be flagged
x = 'foo\
bar'

y = "foo\
bar"

# Backslash inside %q string should not be flagged
z = %q{foo\
bar}

# Backslash inside regexp should not be flagged
r = /foo\
bar/

# Backslash inside backtick string should not be flagged
c = `echo hello\
world`
