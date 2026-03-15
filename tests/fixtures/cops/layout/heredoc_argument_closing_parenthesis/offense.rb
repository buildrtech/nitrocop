foo(<<~SQL
  SELECT * FROM t
SQL
)
^ Layout/HeredocArgumentClosingParenthesis: Put the closing parenthesis for a method call with a HEREDOC parameter on the same line as the HEREDOC opening.

bar(<<~RUBY
  puts 'hello'
RUBY
)
^ Layout/HeredocArgumentClosingParenthesis: Put the closing parenthesis for a method call with a HEREDOC parameter on the same line as the HEREDOC opening.

baz(<<~TEXT
  Some text here
TEXT
)
^ Layout/HeredocArgumentClosingParenthesis: Put the closing parenthesis for a method call with a HEREDOC parameter on the same line as the HEREDOC opening.

# Heredoc inside keyword hash with other keyword args after heredoc body
foo(
  content: <<-EOF,
    This is a heredoc
  EOF
  another_arg: 4,
)
^ Layout/HeredocArgumentClosingParenthesis: Put the closing parenthesis for a method call with a HEREDOC parameter on the same line as the HEREDOC opening.

# Heredoc inside keyword hash with trailing keyword arg
bar(
  description: <<~TEXT,
    Some description
  TEXT
  count: 10,
)
^ Layout/HeredocArgumentClosingParenthesis: Put the closing parenthesis for a method call with a HEREDOC parameter on the same line as the HEREDOC opening.
