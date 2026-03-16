class Foo
  public
  ^^^^^^ Lint/UselessAccessModifier: Useless `public` access modifier.

  def method
  end
end

class Bar
  private
  ^^^^^^^ Lint/UselessAccessModifier: Useless `private` access modifier.
end

class Baz
  protected
  ^^^^^^^^^ Lint/UselessAccessModifier: Useless `protected` access modifier.
end

module Qux
  private
  ^^^^^^^ Lint/UselessAccessModifier: Useless `private` access modifier.

  def self.singleton_method
  end
end

# private_class_method without arguments is useless
class WithPrivateClassMethod
  private_class_method
  ^^^^^^^^^^^^^^^^^^^^ Lint/UselessAccessModifier: Useless `private_class_method` access modifier.

  def self.calculate_something(data)
    data
  end
end
