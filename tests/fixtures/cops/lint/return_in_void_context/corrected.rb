class Foo
  def initialize
    return
  end
end

class Bar
  def initialize(x)
    return if x.nil?
  end
end

class Baz
  def initialize
    return
  end
end

class WithSetter
  def foo=(bar)
    return
  end
end

# return with value inside a regular block within initialize (still an offense)
class WithBlock
  def initialize
    items.each do
      return
    end
  end
end

# return with value inside proc within initialize (still an offense - proc doesn't change scope)
class WithProc
  def initialize
    proc do
      return
    end
  end
end

class WithIndexWriter
  def []=(key, value)
    return
  end
end

class WithClassSetter
  def self.mode=(value)
    return
  end
end

class WithClassIndexWriter
  def self.[]=(key, value)
    return
  end
end
