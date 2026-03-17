::User
::User::Login
::Foo::Bar
::Config
x = 42
y = "hello"
# Fully qualified constants are always fine
::ApplicationRecord
::ActiveRecord::Base
# Class/module definitions should not be flagged
class Foo; end
module Bar; end
class Baz < ::ActiveRecord::Base; end
# Unqualified superclass constants should not be flagged
# (RuboCop skips all direct child constants of class/module nodes)
class AddButtonComponent < ApplicationComponent; end
class ShowPageHeaderComponent < ApplicationComponent; end
class MyModel < ActiveRecord; end
# Single-statement class/module body: constant is the sole body expression,
# so in RuboCop AST the constant is a direct child of the class node and
# `node.parent.defined_module` returns truthy — skipped.
class RaisesNameError
  FooBarBaz
end
class CrossSiteDepender
  CrossSiteDependency
end
module SingleBody
  SomeConst
end
# ConstantPathWriteNode with Class.new/Module.new RHS: target is a module
# definition. RuboCop's `defined_module` returns truthy, so target is suppressed.
# Use fully qualified receivers/args so only the target path matters.
Validators::Custom = ::Module.new
Registry::Entry = ::Class.new(::Base) do
  def call; end
end
