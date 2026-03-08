Struct.new(:name, :age)

Hash.new

x = { name: "John" }

MyClass.new

person = Struct.new(:name).new("John")

# Namespaced OpenStruct is not the stdlib one
YARD::OpenStruct.new
MyModule::OpenStruct.new(a: 1)
Foo::Bar::OpenStruct.new
class A < SomeNamespace::OpenStruct; end

# Reopening/defining OpenStruct class — not a usage
class OpenStruct
  def custom_method
  end
end

module SomeNamespace
  class OpenStruct
  end
end

module SomeNamespace
  module OpenStruct
  end
end
