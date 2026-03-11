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

# When module is NOT the sole top-level statement, RuboCop does not
# unwrap it — so describe with string inside should not be flagged.
require 'spec_helper'

module Foo
  describe '#bar' do
    it { expect(true).to be true }
  end
end

# Multiple top-level statements: require + class wrapper
require 'rails_helper'

class MyTest
  describe 'something' do
    it 'works' do
    end
  end
end

# Two modules at top level — neither should be unwrapped
module Alpha
  describe 'alpha feature' do
    it 'works' do
    end
  end
end

module Beta
  describe 'beta feature' do
    it 'works' do
    end
  end
end
