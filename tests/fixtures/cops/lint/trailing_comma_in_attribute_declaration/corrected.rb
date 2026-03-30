class Foo
  attr_reader :foo

  def bar
    puts "Unreachable."
  end
end

class Bar
  attr_accessor :x

  def baz
    puts "Unreachable."
  end
end

class Baz
  attr_writer :y

  def qux
    puts "Unreachable."
  end
end
