class MyClass
  attr_reader :foo

  attr_reader :bar

  attr_writer :baz

  # class methods (def self.foo) should be flagged by default
  def self.config
    @config
  end

  def self.config=(val)
    @config = val
  end
end

# Methods inside blocks (describe, Class.new, etc.) should be flagged
describe "something" do
  def app
    @app
  end
end

# Methods inside nested blocks should be flagged
describe "outer" do
  context "inner" do
    def name
      @name
    end
  end
end

# Singleton methods on objects inside blocks should be flagged
describe "test" do
  obj = Object.new
  def obj.status
    @status
  end
end

# Top-level defs are only exempt when they are the sole root statement
queue = Object.new

def queue.error
  @error
end

@name = nil

def name
  @name
end

def name=(value)
  @name = value
end

class Camera2D
  def offset = @offset # standard:disable Style/TrivialAccessors

  def target = @target # standard:disable Style/TrivialAccessors
end

class RenderTexture
  def texture = @texture # standard:disable Style/TrivialAccessors
end

Module.new do
  @impl_class = Object

  def self.impl_class
    @impl_class
  end
end

Module.new do
  @klass_name = String

  def self.klass_name
    @klass_name
  end
end
