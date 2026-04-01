foo(<<~SQL)
  SELECT * FROM t
SQL

bar(<<~RUBY)
  puts 'hello'
RUBY

baz(<<~TEXT)
  Some text here
TEXT

# Heredoc inside keyword hash with other keyword args after heredoc body
foo(
  content: <<-EOF,)
    This is a heredoc
  EOF
  another_arg: 4,

# Heredoc inside keyword hash with trailing keyword arg
bar(
  description: <<~TEXT,)
    Some description
  TEXT
  count: 10,
