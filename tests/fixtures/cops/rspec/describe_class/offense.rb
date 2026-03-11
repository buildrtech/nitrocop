describe "bad describe" do
         ^^^^^^^^^^^^^^ RSpec/DescribeClass: The first argument to describe should be the class or module being tested.
end

describe "bad describe", "blah blah" do
         ^^^^^^^^^^^^^^ RSpec/DescribeClass: The first argument to describe should be the class or module being tested.
end

describe 'foo bar', foo: :bar do
         ^^^^^^^^^ RSpec/DescribeClass: The first argument to describe should be the class or module being tested.
end

module MyModule
  describe 'something' do
           ^^^^^^^^^^^ RSpec/DescribeClass: The first argument to describe should be the class or module being tested.
  end
end

module Outer
  module Inner
    RSpec.describe 'another thing' do
                   ^^^^^^^^^^^^^^^ RSpec/DescribeClass: The first argument to describe should be the class or module being tested.
    end
  end
end

class MyClass
  describe 'in a class' do
           ^^^^^^^^^^^^ RSpec/DescribeClass: The first argument to describe should be the class or module being tested.
  end
end

describe 'wow', blah, type: :wow do
         ^^^^^ RSpec/DescribeClass: The first argument to describe should be the class or module being tested.
end
