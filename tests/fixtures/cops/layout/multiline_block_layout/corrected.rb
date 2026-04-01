blah do |i|
  foo(i)
  bar(i)
end

blah { |i|
  foo(i)
  bar(i)
}

items.each do |x|
  process(x)
  finalize(x)
end

blah do |i|
  foo(i)
  bar(i)
rescue
  nil
end

# Lambda with body on same line as opening brace
html = -> {
  content
  more_content
}

# Lambda with params and body on same line
transform = ->(x) {
  x + 1
  y = x * 2
}

# Lambda do..end with body on same line
action = -> do
  run_task
  cleanup
end

# Lambda with heredoc body on same line as opening brace
render -> {
  <<~HTML
<p>hello</p>
HTML
}

# Lambda with method call body on same line
process = -> {
  transform(data)
  finalize(data)
}
