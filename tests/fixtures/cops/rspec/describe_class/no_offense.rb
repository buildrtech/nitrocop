RSpec.describe Foo do
  it 'works' do
  end
end

describe Some::Class do
  describe "bad describe" do
  end
end

RSpec.describe do
end

module MyModule
  describe Some::Class do
    describe "bad describe" do
    end
  end
end

::RSpec.describe Foo do
end

describe 'Thing' do
  subject { Object.const_get(self.class.description) }
end

describe 'Some::Thing' do
  subject { Object.const_get(self.class.description) }
end

describe '::Some::VERSION' do
  subject { Object.const_get(self.class.description) }
end

shared_examples 'Common::Interface' do
  describe '#public_interface' do
    it 'conforms to interface' do
    end
  end
end

RSpec.shared_context 'Common::Interface' do
  describe '#public_interface' do
    it 'conforms to interface' do
    end
  end
end

shared_context do
  describe '#public_interface' do
    it 'conforms to interface' do
    end
  end
end
