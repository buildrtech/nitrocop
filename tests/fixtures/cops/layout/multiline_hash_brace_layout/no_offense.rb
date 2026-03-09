x = { a: 1,
      b: 2 }

y = {
  a: 1,
  b: 2
}

z = { a: 1, b: 2 }

h = {
  one: 1,
  two: 2,
  three: 3
}

# Heredoc as last value — closing brace on separate line is allowed
config = {
  name: "test",
  description: <<~DESC
    Some description
    across multiple lines
  DESC
}

data = { key: "value",
         body: <<~BODY
           heredoc content
         BODY
}
