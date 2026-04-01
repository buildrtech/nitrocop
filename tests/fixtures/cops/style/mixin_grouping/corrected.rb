class Foo
  include Bar
  include Qux
end

class Baz
  extend A
  extend B
end

class Quux
  prepend X
  prepend Y
  prepend Z
end
