[1, 2, 3]
[:a, :b]
x = ["foo"]
# Multiline array: space after [ when elements on same line (no_space default)
[Element::Form, Element::Link,
  Element::Cookie]
# Multiple spaces after opening bracket
[1, 2, 3]
# Multiple spaces before closing bracket
[1, 2, 3]
# Empty brackets with multiple spaces (should be empty offense)
[]
# Empty brackets with newline (multiline empty)
[]
