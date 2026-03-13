%w[foo bar baz]

['foo']

['foo bar', 'baz']

[1, 2, 3]

%w[one two]

[]

# Array with empty string — can't be represented in %w
['es', 'fr', '']

# Single-quoted string with literal backslash — not a word
['has\nescapes', 'foo']

# Non-word strings (hyphens only, not connecting word chars)
['-', '----']

# Array with comments inside
[
"foo", # a comment
"bar", # another comment
"baz"  # trailing comment
]

# Array with space in a string
["one space", "two", "three"]

# Matrix of complex content: parent array where all elements are arrays and
# at least one subarray has complex content (space in "United States").
# RuboCop exempts all subarrays in such a matrix.
[
  ["US", "United States"],
  ["UK", "United Kingdom"],
  ["CA", "Canada"]
]

# Matrix with all-word subarrays but one has a space
[
  ["AL", "Albania"],
  ["AS", "American Samoa"],
  ["AD", "Andorra"]
]

# Simple 2-element matrix where one pair has space
[["foo", "bar"], ["baz quux", "qux"]]
