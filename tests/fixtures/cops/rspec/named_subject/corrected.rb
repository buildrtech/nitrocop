RSpec.describe User do
  subject { described_class.new }

  it "is valid" do
    expect(skip('TODO: name subject explicitly').valid?).to be(true)
  end

  specify do
    expect(skip('TODO: name subject explicitly').valid?).to be(true)
  end

  before(:each) do
    do_something_with(skip('TODO: name subject explicitly'))
  end

  # subject { ... } inside a hook is a reference, not a definition
  around(:each) do |example|
    skip('TODO: name subject explicitly')
    example.run
  end

  after do
    do_something_with(skip('TODO: name subject explicitly'))
  end

  # subject inside a block within an example is still flagged
  it "does not raise" do
    expect { skip('TODO: name subject explicitly') }.not_to raise_error
  end

  # prepend_before is also a hook
  prepend_before do
    setup(skip('TODO: name subject explicitly'))
  end

  # def subject.method_name — the `subject` receiver is a test subject reference
  it "overrides behavior" do
    def skip('TODO: name subject explicitly').custom_method
      true
    end
    skip('TODO: name subject explicitly').custom_method
  end
end

# `it` blocks inside helper methods — subject inside them should be flagged
def self.it_should_have_view(key, value)
  it "should have #{value} for view key '#{key}'" do
    expect(skip('TODO: name subject explicitly').send(key)).to eq value
  end
end

# shared_context is NOT a shared example group for IgnoreSharedExamples purposes.
# RuboCop's `shared_example?` matcher only matches SharedGroups.examples
# (shared_examples, shared_examples_for), not SharedGroups.context (shared_context).
# So subject inside shared_context examples IS flagged even with IgnoreSharedExamples: true.
RSpec.describe Config do
  subject { described_class.new }

  shared_context "with setup" do
    it "succeeds" do
      expect(skip('TODO: name subject explicitly').status).to eq(200)
    end
  end
end
