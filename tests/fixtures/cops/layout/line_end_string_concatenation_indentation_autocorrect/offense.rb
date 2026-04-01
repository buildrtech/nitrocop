text = 'hello' \
    'world'
    ^^^^^^^ Layout/LineEndStringConcatenationIndentation: Align parts of a string concatenated with backslash.

def some_method
  'x' \
  'y'
  ^^^ Layout/LineEndStringConcatenationIndentation: Indent the first part of a string concatenated with backslash.
end
