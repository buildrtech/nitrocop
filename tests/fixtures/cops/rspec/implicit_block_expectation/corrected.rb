# Lambda subject with is_expected and change matcher
describe 'command' do
  subject { -> { run_command } }
  it { skip('TODO: avoid implicit block expectations').to change { something }.to(new_value) }
end

# Lambda subject with custom matcher (the key FN fix)
describe 'termination' do
  subject { -> { described_class.run(args) } }
  it { skip('TODO: avoid implicit block expectations').to terminate }
end

# subject! with lambda
describe 'eager' do
  subject! { -> { boom } }
  it { skip('TODO: avoid implicit block expectations').to terminate }
end

# proc {} subject
describe 'proc subject' do
  subject { proc { do_something } }
  it { skip('TODO: avoid implicit block expectations').to be_valid }
end

# lambda {} subject
describe 'lambda subject' do
  subject { lambda { do_something } }
  it { skip('TODO: avoid implicit block expectations').to eq(result) }
end

# Proc.new {} subject
describe 'proc new subject' do
  subject { Proc.new { do_something } }
  it { skip('TODO: avoid implicit block expectations').to change { something }.to(new_value) }
end

# Named subject with lambda
describe 'named' do
  subject(:action) { -> { boom } }
  it { skip('TODO: avoid implicit block expectations').to change { something }.to(new_value) }
end

# Lambda subject inherited from outer group
describe 'outer' do
  subject { -> { boom } }
  context 'inner' do
    it { skip('TODO: avoid implicit block expectations').to change { something }.to(new_value) }
  end
end

# should / should_not with lambda subject
describe 'should variants' do
  subject { -> { boom } }
  it { skip('TODO: avoid implicit block expectations') }
  it { skip('TODO: avoid implicit block expectations') }
end

# Multiple examples with lambda subject
describe 'multiple' do
  subject { -> { described_class.run(args) } }
  context 'missing file' do
    let(:file) { 'missing.rb' }
    it { skip('TODO: avoid implicit block expectations').to terminate.with_code(1) }
  end
  context 'unchanged file' do
    let(:file) { 'spec/fixtures/valid' }
    it { skip('TODO: avoid implicit block expectations').to terminate }
  end
end

# Custom method (its_call) with is_expected inside a lambda-subject group
# RuboCop flags is_expected in ANY block within an example group that has a lambda subject,
# not just blocks of known example methods like it/specify.
describe 'custom method block' do
  subject { -> { process(input) } }
  its_call('value') { skip('TODO: avoid implicit block expectations').to ret([result]) }
end

# Custom method with should inside lambda-subject group
describe 'custom should' do
  subject { -> { run_action } }
  its_call('arg') { skip('TODO: avoid implicit block expectations') }
end

# Custom method inheriting lambda subject from parent group
describe 'inherited subject' do
  subject { -> { execute } }
  context 'nested' do
    its_call('test') { skip('TODO: avoid implicit block expectations').to terminate }
  end
end

# RSpec.describe with explicit receiver — top-level example group with RSpec. prefix
RSpec.describe 'with receiver' do
  subject { ->(source) { process(source) } }
  its_call('value') { skip('TODO: avoid implicit block expectations').to ret([result]) }
end

# RSpec.describe with lambda subject and regular it block
RSpec.describe 'receiver with it' do
  subject { -> { boom } }
  it { skip('TODO: avoid implicit block expectations').to change { something }.to(new_value) }
end

# RSpec.describe with nested context inheriting lambda subject
RSpec.describe 'receiver nested' do
  subject { -> { execute } }
  context 'inner' do
    it { skip('TODO: avoid implicit block expectations').to terminate }
  end
end
