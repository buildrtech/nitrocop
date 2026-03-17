class Foo
  include Bar

  attr_reader :baz
end

class Qux
  extend ActiveSupport::Concern

  def some_method
  end
end

class Abc
  prepend MyModule

  def another_method
  end
end

# include inside multi-statement block (Class.new, RSpec.describe, etc.)
Class.new do
  include AccountableConcern

  attr_reader :current_account
  def initialize
  end
end

RSpec.describe User do
  include RSpec::Rails::RequestExampleGroup

  let(:username) { 'alice' }
  it 'does something' do
  end
end

# include inside class nested within if block (class resets if context)
if some_condition
  class Child
    include Serializable

    attr_reader :data
  end
end

require "support/helpers"

include Support::Helpers

records = build_records

def setup
  include MyHelper

  do_stuff
end

# include inside multi-statement if body (parent is begin, not if)
class Config
  if RUBY_VERSION >= '1.9'
    include Comparable

    def <=>(other)
      name <=> other.name
    end
  end
end

# extend inside multi-statement if body
class Worker
  if feature_enabled?
    extend Forwardable

    def_delegator :config, :timeout
  end
end

# include inside begin...rescue block
class Service
  begin
    include Serializable

    validate :check_format
  rescue NameError
    use_fallback
  end
end

# include with rescue modifier followed by non-include code
class Provider
  include Logging rescue LoadError

  validate :check_config
end

# extend with rescue modifier followed by non-include code
module Helpers
  extend Formatting rescue NameError

  def setup; end
end
