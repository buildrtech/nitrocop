[1, 2, 3]

["a", "b"]

[:foo, :bar]

# Multiline array with trailing comma and blank line before closing bracket
[
  1,
  2

]

# Multiline array with trailing comma and comment before closing bracket
[
  "x",
  "y" # a comment

]

# Heredoc as last element with trailing comma (FN fix)
x = [
  "foo",
  <<~STR.chomp
    content here
  STR
]

# Heredoc as last element with trailing comma (no method chain)
y = [
  "bar",
  <<~STR
    more content
  STR
]

# Heredoc with squiggly heredoc and trailing comma
z = [
  "baz",
  <<~HEREDOC
    some text
  HEREDOC
]
