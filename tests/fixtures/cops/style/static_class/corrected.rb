module Foo
  def self.bar
    42
  end
end

module Bar
  def self.baz
    'hello'
  end
  def self.qux
    'world'
  end
end

module Utils
  def self.helper
    true
  end
end

module WithConstant
  CONST = 1
  def self.foo
    CONST
  end
end

module WithExtend
  extend SomeModule
  def self.class_method; end
end

module WithSclass
  def self.class_method; end

  class << self
    def other_class_method; end
  end
end

module WithSclassAssignment
  class << self
    SETTING = 1
    def configure; end
  end
end

module WithEmptySclass
  class << self
  end
end

module WithMultiWrite
  VERSION = "4.2.8"
  MAJOR, MINOR, TINY = VERSION.split(".")
end

module WithOnlyMultiWrite
  MAJOR, MINOR, TINY = "1.2.3".split(".")
end

module Wrapper
  module NestedStatic
    @@edited = nil
    @@switch_index = 0
    @@dash_prefix, @@dash_suffix = nil, nil
  end
end
