module Outer
  module Inner
    RSpec.describe 'another thing' do
                   ^^^^^^^^^^^^^^^ RSpec/DescribeClass: The first argument to describe should be the class or module being tested.
    end
  end
end
