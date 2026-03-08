RSpec.describe Foo do
  describe '#bar' do
  end
  ^^^ RSpec/EmptyLineAfterExampleGroup: Add an empty line after `describe`.
  describe '#baz' do
  end

  context 'first' do
  end
  ^^^ RSpec/EmptyLineAfterExampleGroup: Add an empty line after `context`.
  context 'second' do
  end

  shared_examples 'foo' do
  end
  ^^^ RSpec/EmptyLineAfterExampleGroup: Add an empty line after `shared_examples`.
  it 'works' do
  end
end

# RSpec-prefixed example groups at top level should also be checked
RSpec.describe 'first' do
end
^^^ RSpec/EmptyLineAfterExampleGroup: Add an empty line after `describe`.
RSpec.describe 'second' do
end

RSpec.shared_examples 'one' do
end
^^^ RSpec/EmptyLineAfterExampleGroup: Add an empty line after `shared_examples`.
RSpec.shared_context 'two' do
end

# Single-line RSpec-prefixed example groups
RSpec.describe('inline') { }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/EmptyLineAfterExampleGroup: Add an empty line after `describe`.
RSpec.describe 'next' do
end
