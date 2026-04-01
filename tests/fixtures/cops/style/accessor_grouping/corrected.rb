class Foo
  attr_reader :bar1, :bar2
  attr_accessor :quux
  attr_reader :bar3, :bar4
  other_macro :zoo
end

# Accessors separated by method definitions are still groupable
class WithDefs
  def foo
  end
  attr_reader :one

  def bar
  end
  attr_reader :two
end

# Non-contiguous accessors separated by blank lines are groupable
class BlankLineSeparated
  attr_reader :alpha

  attr_reader :beta
end

# Accessors after bare visibility modifiers are groupable within scope
class WithVisibility
  attr_reader :pub1, :pub2

  private
  attr_reader :priv1, :priv2
  attr_writer :w1
  attr_reader :priv3

  public
  attr_reader :pub3
  other_macro :zoo
end

# Accessors after annotation method + blank line are still groupable with others
class AfterAnnotation
  extend T::Sig

  sig { returns(Integer) }
  attr_reader :one

  attr_reader :two, :three

  attr_reader :four
end

# Accessors after block-form DSL calls are grouped by the call line, not the block end
class AfterBlockMacro
  mattr_accessor :items do
    []
  end
  attr_reader :name, :url, :enabled
end

# Block-form config DSLs behave the same way as other block calls
class AfterConfigSection
  config_section :client, param_name: :clients do
    config_param :host, :string, default: nil
  end
  attr_reader :nodes

  attr_reader :sessions
end
