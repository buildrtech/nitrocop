class Foo
  attr_reader :bar
              ^^^^ Style/BisectedAttrAccessor: Combine both accessors into `attr_accessor :bar`.
  attr_writer :bar
              ^^^^ Style/BisectedAttrAccessor: Combine both accessors into `attr_accessor :bar`.
  other_macro :something
end

class Baz
  attr_reader :qux
              ^^^^ Style/BisectedAttrAccessor: Combine both accessors into `attr_accessor :qux`.
  attr_writer :qux
              ^^^^ Style/BisectedAttrAccessor: Combine both accessors into `attr_accessor :qux`.
end

# Same visibility scope (both public, then both private)
class SameVisibility
  attr_reader :foo
              ^^^^ Style/BisectedAttrAccessor: Combine both accessors into `attr_accessor :foo`.
  attr_writer :foo
              ^^^^ Style/BisectedAttrAccessor: Combine both accessors into `attr_accessor :foo`.

  private

  attr_writer :baz
              ^^^^ Style/BisectedAttrAccessor: Combine both accessors into `attr_accessor :baz`.
  attr_reader :baz
              ^^^^ Style/BisectedAttrAccessor: Combine both accessors into `attr_accessor :baz`.
end

# Within eigenclass
class WithEigenclass
  attr_reader :bar

  class << self
    attr_reader :baz
                ^^^^ Style/BisectedAttrAccessor: Combine both accessors into `attr_accessor :baz`.
    attr_writer :baz
                ^^^^ Style/BisectedAttrAccessor: Combine both accessors into `attr_accessor :baz`.

    private

    attr_reader :quux
  end
end

module SomeModule
  attr_reader :name
              ^^^^^ Style/BisectedAttrAccessor: Combine both accessors into `attr_accessor :name`.
  attr_writer :name, :role
              ^^^^^ Style/BisectedAttrAccessor: Combine both accessors into `attr_accessor :name`.
end
