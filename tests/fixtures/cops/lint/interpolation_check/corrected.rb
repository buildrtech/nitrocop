x = "hello #{name}"

y = "value: #{foo}"

z = "#{bar}"

w = "THIS. IS. #{yield.upcase}!"

# String containing double quotes — RuboCop flags this (corrects with %{})
a = "foo \"#{bar}\""

# Split string where the second part has interpolation
"x" \
  "foo #{bar}"

# %q strings with interpolation patterns — RuboCop flags these (single-line only)
a = "hello #{name}"

b = "text #{foo} bar"

c = "value #{bar}"

# Strings with non-standard uppercase escapes like \A, \Z — the Parser gem
# does NOT fatally reject these (only \U is fatal). So these are valid
# interpolation candidates and should be flagged.
d = "\\A#{regexp}\\z"

e = "\\Astring#{interpolation}\\z"
