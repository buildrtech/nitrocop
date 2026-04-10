describe MyClass do
  before { @foo = [] }
  it { expect(skip('TODO: avoid instance variable')).to be_empty }
  it { expect(skip('TODO: avoid instance variable')).to be_empty }
end

# Reads inside shared examples are flagged
shared_examples 'shared example' do
  it { expect(skip('TODO: avoid instance variable')).to be_empty }
end

# Multiple reads in different example blocks
describe AnotherClass do
  before { @app = create(:app) }
  it 'reads in example' do
    expect(skip('TODO: avoid instance variable').name).to eq('test')
  end
  it 'also reads' do
    expect(skip('TODO: avoid instance variable')).to be_valid
  end
end

# Reads inside helper methods (def) within describe blocks ARE flagged
describe HelperMethods do
  def helper
    skip('TODO: avoid instance variable')
  end
end

# Reads inside Struct.new blocks ARE flagged (only Class.new is excluded)
describe StructNewExample do
  let(:klass) do
    Struct.new(:name) do
      def display
        skip('TODO: avoid instance variable')
      end
    end
  end
end

# Reads inside module_eval/class_eval blocks ARE flagged
describe EvalBlocks do
  before do
    described_class.class_eval do
      skip('TODO: avoid instance variable')
    end
  end
end

# RSpec.shared_examples with explicit receiver is a top-level group
RSpec.shared_examples 'shared behavior' do
  it { expect(skip('TODO: avoid instance variable')).to be_valid }
end

# RSpec.shared_context with explicit receiver is a top-level group
RSpec.shared_context 'with setup' do
  before { @config = build(:config) }
  it { skip('TODO: avoid instance variable').valid? }
end

# RSpec.context with explicit receiver is a top-level group
RSpec.context 'standalone context' do
  it { skip('TODO: avoid instance variable') }
end

# Instance variables inside matcher blocks with variable argument ARE flagged
# (only symbol-argument matchers are excluded, matching RuboCop's NodePattern)
describe MatcherWithVariable do
  %i[create_item update_item].each do |key|
    matcher key do |opts = {}|
      match do |*|
        @body = requests[key]
        skip('TODO: avoid instance variable').present?
      end
      failure_message do
        "Expected #{key} but got #{skip('TODO: avoid instance variable')}"
      end
    end
  end
end
