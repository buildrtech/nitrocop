class Foo
  attr_accessor :bar
  attr_reader :baz
  attr_writer :qux
  other_macro :something
end

class Bar
  attr_reader :x
end

# Different visibility scopes should not be bisected
class DifferentVisibility
  attr_reader :bar

  private
  attr_writer :bar
end

# Protected vs public should not be bisected
class ProtectedVsPublic
  attr_reader :name

  protected
  attr_writer :name
end

# Private reader + public writer
class PrivateReaderPublicWriter
  private
  attr_reader :secret

  public
  attr_writer :secret
end

# Eigenclass (class << self)
class WithEigenclass
  attr_reader :bar

  class << self
    attr_reader :baz
    attr_writer :qux
  end
end
