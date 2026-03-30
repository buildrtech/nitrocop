class Foo
  def initialize
    @x = 1
  end
end

class Bar
  def initialize
    @a = []
    @b = {}
  end
end

# Leading ||= followed by non-||= should still flag the leading ones
class Baz
  def initialize
    @delicious = true
    super
  end
end
