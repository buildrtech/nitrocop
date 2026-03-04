require "spec_helper"

describe Foo do
  subject(:foo) { described_class.new }

  before do
    allow(other_obj).to receive(:bar).and_return(baz)
  end

  it 'does something' do
    expect(foo.bar).to eq(baz)
  end
end

describe Bar do
  let(:bar) { double }

  before do
    allow(bar).to receive(:baz)
  end
end

# When require is at top level alongside a module wrapper, RuboCop's TopLevelGroup
# does not recurse into the module (begin returns children directly, module is
# not a spec group so it is skipped).
module SomeModule
  describe Builder do
    subject { described_class.new }

    before do
      allow(subject).to receive(:windows?)
    end
  end
end

# Local variable named subject is not the RSpec subject method
describe Agent do
  it 'returns false when failed?' do
    subject = Agent.new(0)
    allow(subject).to receive(:failed?).and_return(true)
    expect(subject.send { nil }).to be false
  end
end
