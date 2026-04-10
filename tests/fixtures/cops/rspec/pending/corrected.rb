# x-prefixed example groups (with or without block)
context 'test' do
end
describe 'test' do
end
feature 'test' do
end

# x-prefixed examples
it 'test' do
end
specify 'test' do
end
example 'test' do
end
scenario 'test' do
end

# pending/skip as example-level calls
pending 'test' do
end
skip 'test' do
end

# standalone skip/pending inside examples
it 'test' do
  skip
end
it 'test' do
  pending
end

# skip/pending with a reason string (no block)
it 'test' do
  skip 'not implemented yet'
end

# symbol metadata on examples
it 'test', :skip do
end
it 'test', :pending do
end

# keyword metadata on examples
it 'test', skip: true do
end
it 'test', skip: 'skipped because of being slow' do
end

# symbol metadata on example groups
RSpec.describe 'test', :skip do
end
context 'test', :pending do
end

# keyword metadata on example groups
describe 'test', skip: true do
end

# examples without bodies (pending by definition)
it 'test'
specify 'test'
example 'test'

# examples with only block-pass args are still body-less in RuboCop
it 'uses a proc body', &(proc do
  expect(true).to be(true)
end)
