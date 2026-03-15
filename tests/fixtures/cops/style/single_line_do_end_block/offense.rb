foo do |x| x end
^^^^^^^^^^^^^^^^ Style/SingleLineDoEndBlock: Prefer multiline `do`...`end` block.

bar do puts 'hello' end
^^^^^^^^^^^^^^^^^^^^^^^ Style/SingleLineDoEndBlock: Prefer multiline `do`...`end` block.

baz do |a, b| a + b end
^^^^^^^^^^^^^^^^^^^^^^^ Style/SingleLineDoEndBlock: Prefer multiline `do`...`end` block.

foo do end
^^^^^^^^^^ Style/SingleLineDoEndBlock: Prefer multiline `do`...`end` block.

foo do bar(_1) end
^^^^^^^^^^^^^^^^^^ Style/SingleLineDoEndBlock: Prefer multiline `do`...`end` block.

->(arg) do foo arg end
^^^^^^^^^^^^^^^^^^^^^^ Style/SingleLineDoEndBlock: Prefer multiline `do`...`end` block.

lambda do |arg| foo(arg) end
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/SingleLineDoEndBlock: Prefer multiline `do`...`end` block.
