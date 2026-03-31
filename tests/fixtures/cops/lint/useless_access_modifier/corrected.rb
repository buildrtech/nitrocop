class Foo

  def method
  end
end

class Bar
end

class Baz
end

module Qux

  def self.singleton_method
  end
end

# private_class_method without arguments is useless
class WithPrivateClassMethod

  def self.calculate_something(data)
    data
  end
end

# top-level access modifiers are always useless

def top_level_method
end


def another_top_level_method
end

# module_function at top level is useless

def top_func
end

# module_function inside a module followed only by eval is useless
module WithModuleFunction
  eval "def test1() end"
end

# module_function repeated inside a module
module RepeatedModuleFunction
  module_function

  def first_func; end


  def second_func; end
end

# useless access modifier inside Class.new do block
Class.new do
end

# FN fix: private repeated due to visibility leaking from conditional branch
# RuboCop's check_child_nodes recurses into if/else, propagating cur_vis
class WithVisibilityFromConditional
  if some_condition
    private

    def secret_method
    end
  end


  def another_method
  end
end

# FN fix: private before class definition + method def, where visibility
# leaked from inside a block making it a repeated modifier
class WithBlockVisibilityLeak
  some_dsl :items do
    property :title

    private

    def populate_item!
      Item.new
    end
  end

  property :artist do
    property :name
  end


  class Helper < Base
    attr_accessor :args
  end

  def create_item(input)
    Helper.new
  end
end
