x = [
  :a, :b,
      ^^ Layout/MultilineArrayLineBreaks: Each item in a multi-line array must start on a separate line.
  :c
]

y = [
  1, 2,
     ^ Layout/MultilineArrayLineBreaks: Each item in a multi-line array must start on a separate line.
  3
]

z = [
  :foo, :bar,
        ^^^^ Layout/MultilineArrayLineBreaks: Each item in a multi-line array must start on a separate line.
  :baz
]

# Rescue exception lists — multiple exceptions on same line in multi-line rescue
begin
  something
rescue FooError, BarError,
                 ^^^^^^^^ Layout/MultilineArrayLineBreaks: Each item in a multi-line array must start on a separate line.
       BazError
  retry
end

begin
  something
rescue FooError, BarError,
                 ^^^^^^^^ Layout/MultilineArrayLineBreaks: Each item in a multi-line array must start on a separate line.
       BazError, QuxError
                 ^^^^^^^^ Layout/MultilineArrayLineBreaks: Each item in a multi-line array must start on a separate line.
  retry
end
