case a
when b then c
end

case e
when f
  g
end

case value
when cond1
end

case x
when 1 then "one"
when 2 then "two"
end

case total
when 1;
  "one"
when 2;
  "two"
end

# Comment with semicolon between when condition and body
case value
when /pattern/
  # Handles text/html; charset="GB2312"
  process(value)
end

case input
when String
  # key; value pairs
  parse(input)
end

# Multiline when condition with semicolon — not flagged
case url
when %r[
  pattern
]x; puts url
end

# Multiline when with heredoc-like multiline condition
case value
when *[
       1,
       2,
       3
     ]; handle(value)
end
