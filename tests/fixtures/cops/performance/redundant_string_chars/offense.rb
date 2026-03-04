str.chars.first
^^^^^^^^^^^^^^^ Performance/RedundantStringChars: Use `[]` instead of `chars.first`.
str.chars[0]
^^^^^^^^^^^^ Performance/RedundantStringChars: Use `[]` instead of `chars.first`.
str.chars.last
^^^^^^^^^^^^^^ Performance/RedundantStringChars: Use `[]` instead of `chars.first`.
str.chars.length
^^^^^^^^^^^^^^^^ Performance/RedundantStringChars: Use `.length` instead of `chars.length`.
str.chars.size
^^^^^^^^^^^^^^ Performance/RedundantStringChars: Use `.size` instead of `chars.size`.
str.chars.empty?
^^^^^^^^^^^^^^^^ Performance/RedundantStringChars: Use `.empty?` instead of `chars.empty?`.
str.chars.first(2)
^^^^^^^^^^^^^^^^^^ Performance/RedundantStringChars: Use `[0...2].chars` instead of `chars.first(2)`.
str.chars.take(2)
^^^^^^^^^^^^^^^^^ Performance/RedundantStringChars: Use `[0...2].chars` instead of `chars.take(2)`.
str.chars.slice(0..2)
^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantStringChars: Use `[0..2].chars` instead of `chars.slice(0..2)`.
