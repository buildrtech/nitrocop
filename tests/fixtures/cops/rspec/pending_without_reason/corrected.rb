RSpec.describe Foo do
  pending('TODO: reason')
  skip('TODO: reason')
  xit 'something' do
  end
  xit('TODO: reason')
end

RSpec.xdescribe 'top level skipped' do
  it 'does something' do
  end
end

RSpec.xcontext 'top level skipped context' do
  it 'does something' do
  end
end

RSpec.describe Foo do
  xdescribe 'nested skipped without RSpec receiver' do
    it 'does something' do
    end
  end
end
