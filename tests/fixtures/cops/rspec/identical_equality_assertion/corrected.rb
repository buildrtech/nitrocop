RSpec.describe 'test' do
  it 'compares with eq' do
    skip('TODO: avoid identical equality assertion').to eq(foo.bar)
  end

  it 'compares with eql' do
    skip('TODO: avoid identical equality assertion').to eql(foo.bar.baz)
  end

  it 'compares trivial constants' do
    skip('TODO: avoid identical equality assertion').to eq(42)
  end

  it 'compares dot vs constant path for lowercase method' do
    skip('TODO: avoid identical equality assertion').to eq(Obj::method_name)
  end

  it 'compares empty array literals' do
    skip('TODO: avoid identical equality assertion').to eq([])
  end

  it 'compares regex with equivalent escapes' do
    skip('TODO: avoid identical equality assertion').to eq(/[§]/)
  end
end
