it 'something' do
  skip('TODO: avoid void expect')
end
it 'another' do
  skip('TODO: avoid void expect')
end
it 'third' do
  x = 1
  skip('TODO: avoid void expect')
end
# Block form of expect (expect { ... })
it 'block form' do
  skip('TODO: avoid void expect')
end
# Block form as sole statement
it 'block form sole' do
  skip('TODO: avoid void expect')
end
# Nested inside describe/context
describe Foo do
  context 'bar' do
    it 'nested void expect' do
      skip('TODO: avoid void expect')
    end
  end
end
# Inside aggregate_failures
it 'test' do
  aggregate_failures do
    skip('TODO: avoid void expect')
  end
end
# Multi-statement if branch: expect's parent is begin_type? -> void
it 'multi-statement if' do
  if condition
    setup
    skip('TODO: avoid void expect')
  end
end
# Parenthesized expect with .to is still void per RuboCop
# (parens create a begin node, making the expect's parent begin_type?)
it 'parenthesized chained' do
  (skip('TODO: avoid void expect')).to be 1
end
it 'parenthesized chained not_to' do
  (skip('TODO: avoid void expect')).not_to eq(2)
end
